// SPDX-License-Identifier: AGPL-3.0-only
//! Synthetic ECG waveform generation for testing and demonstration.
//!
//! Each cardiac cycle is modeled as a sequence of frequency segments:
//! P wave (atrial depolarization), QRS complex (ventricular depolarization),
//! T wave (ventricular repolarization), and an RR gap (diastolic interval).
//!
//! The QRS complex is aligned to 4500 Hz — the CricketBrain carrier frequency —
//! so the coincidence detector fires on each ventricular beat.

/// A single cardiac cycle as a sequence of (frequency_hz, duration_ms) segments.
#[derive(Debug, Clone)]
pub struct EcgCycle {
    pub segments: Vec<(f32, usize)>,
}

impl EcgCycle {
    /// Total duration of one cycle in milliseconds.
    pub fn duration_ms(&self) -> usize {
        self.segments.iter().map(|(_, d)| d).sum()
    }

    /// Instantaneous heart rate in BPM.
    pub fn bpm(&self) -> f32 {
        let dur = self.duration_ms();
        if dur == 0 {
            return 0.0;
        }
        60_000.0 / dur as f32
    }

    /// Convert N repetitions of this cycle into a flat frequency stream.
    /// Each element represents one millisecond timestep.
    pub fn to_frequency_stream(&self, n_cycles: usize) -> Vec<f32> {
        let mut stream = Vec::with_capacity(self.duration_ms() * n_cycles);
        for _ in 0..n_cycles {
            for &(freq, dur) in &self.segments {
                stream.extend(std::iter::repeat(freq).take(dur));
            }
        }
        stream
    }
}

// --- Waveform constants (from sentinel_ecg_monitor.rs analysis) ---
// P wave:  3100 Hz, 12 ms — atrial depolarization
// QRS:     4500 Hz, 10 ms — ventricular depolarization (carrier-aligned)
// T wave:  3400 Hz, 14 ms — ventricular repolarization
// Gaps:    0 Hz (silence between wave components)

const P_FREQ: f32 = 3100.0;
const P_DUR: usize = 12;
const QRS_FREQ: f32 = 4500.0;
const QRS_DUR: usize = 10;
const T_FREQ: f32 = 3400.0;
const T_DUR: usize = 14;
const INTER_WAVE_GAP: usize = 4;

// P+gap+QRS+gap+T = 12+4+10+4+14 = 44 ms of wave activity.
// RR interval = total cycle duration. Gap = RR - 44 ms.
const WAVE_DUR: usize = P_DUR + INTER_WAVE_GAP + QRS_DUR + INTER_WAVE_GAP + T_DUR; // 44 ms

fn make_cycle(rr_gap_ms: usize) -> EcgCycle {
    EcgCycle {
        segments: vec![
            (P_FREQ, P_DUR),
            (0.0, INTER_WAVE_GAP),
            (QRS_FREQ, QRS_DUR),
            (0.0, INTER_WAVE_GAP),
            (T_FREQ, T_DUR),
            (0.0, rr_gap_ms),
        ],
    }
}

/// Normal sinus rhythm (~73 BPM). RR interval ~820 ms, gap = 776 ms.
pub fn normal_sinus() -> EcgCycle {
    make_cycle(820 - WAVE_DUR) // 776 ms diastolic gap
}

/// Tachycardia (~150 BPM). RR interval ~400 ms, gap = 356 ms.
pub fn tachycardia() -> EcgCycle {
    make_cycle(400 - WAVE_DUR) // 356 ms diastolic gap
}

/// Bradycardia (~40 BPM). RR interval ~1500 ms, gap = 1456 ms.
pub fn bradycardia() -> EcgCycle {
    make_cycle(1500 - WAVE_DUR) // 1456 ms diastolic gap
}

// ---------------------------------------------------------------------------
// CSV I/O for preprocessed data
// ---------------------------------------------------------------------------

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// A single preprocessed beat record from CSV.
///
/// `record_id` was added in v0.3 and identifies which MIT-BIH-style
/// patient record the beat came from. The legacy 5-column CSV format
/// (without `record_id`) is still accepted by [`from_csv`]; missing
/// `record_id` defaults to the empty string.
#[derive(Debug, Clone)]
pub struct BeatRecord {
    pub timestamp_ms: f32,
    pub rr_interval_ms: f32,
    pub beat_type: String,
    pub bpm: f32,
    pub mapped_freq: f32,
    /// Patient / record identifier (e.g. MIT-BIH "100"). Empty for
    /// legacy CSVs that don't carry the column.
    pub record_id: String,
}

/// Read a preprocessed CSV.
///
/// Accepts both the legacy 5-column header
/// `timestamp_ms,rr_interval_ms,beat_type,bpm,mapped_freq` and the
/// v0.3 6-column header
/// `timestamp_ms,rr_interval_ms,beat_type,bpm,mapped_freq,record_id`.
/// In the legacy case `record_id` is filled with the file stem (so
/// per-record aggregation still works).
pub fn from_csv(path: &str) -> Vec<BeatRecord> {
    let file = fs::File::open(path).expect("cannot open CSV");
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    let fallback_id = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let mut header_has_record_id = false;
    for (i, line) in reader.lines().enumerate() {
        let line = line.expect("cannot read line");
        if i == 0 {
            header_has_record_id = line.split(',').any(|c| c.trim() == "record_id");
            continue;
        }
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() < 5 {
            continue;
        }
        let rid = if header_has_record_id && cols.len() >= 6 {
            cols[5].trim().to_string()
        } else {
            fallback_id.clone()
        };
        records.push(BeatRecord {
            timestamp_ms: cols[0].parse().unwrap_or(0.0),
            rr_interval_ms: cols[1].parse().unwrap_or(0.0),
            beat_type: cols[2].to_string(),
            bpm: cols[3].parse().unwrap_or(0.0),
            mapped_freq: cols[4].parse().unwrap_or(0.0),
            record_id: rid,
        });
    }

    records
}

/// Recursively load every `*.csv` under `dir` and return them grouped
/// by `record_id`. Each `(record_id, beats)` group is sorted by
/// `timestamp_ms` so a stream that was split across files re-assembles
/// correctly.
///
/// Files that fail to parse or don't end in `.csv` are skipped. The
/// scan is non-recursive (only files directly in `dir`); deeper
/// scanning is intentionally not done here so that
/// `data/processed/train/` and `data/processed/test/` stay separate
/// when the caller asks for them separately.
pub fn from_csv_dir(dir: &str) -> Vec<(String, Vec<BeatRecord>)> {
    let mut grouped: std::collections::BTreeMap<String, Vec<BeatRecord>> =
        std::collections::BTreeMap::new();

    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("csv") {
            continue;
        }
        let path_str = match path.to_str() {
            Some(s) => s,
            None => continue,
        };
        let beats = from_csv(path_str);
        for b in beats {
            grouped.entry(b.record_id.clone()).or_default().push(b);
        }
    }

    let mut out: Vec<(String, Vec<BeatRecord>)> = grouped.into_iter().collect();
    for (_, beats) in &mut out {
        beats.sort_by(|a, b| {
            a.timestamp_ms
                .partial_cmp(&b.timestamp_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    out
}

/// Convert beat records to a CricketBrain frequency stream.
/// Each beat produces its mapped_freq for the RR interval duration,
/// then a QRS burst at 4500 Hz for 10 ms.
pub fn beats_to_frequency_stream(beats: &[BeatRecord]) -> Vec<f32> {
    let mut stream = Vec::new();
    for beat in beats {
        // Diastolic gap at silence, then QRS burst
        let gap_ms = (beat.rr_interval_ms as usize).saturating_sub(10);
        stream.extend(std::iter::repeat(0.0f32).take(gap_ms));
        stream.extend(std::iter::repeat(QRS_FREQ).take(10)); // QRS spike
    }
    stream
}

/// Write synthetic sample CSV for testing without real data.
pub fn write_sample_csv(path: &str, n_per_class: usize) {
    let parent = Path::new(path).parent().expect("invalid path");
    fs::create_dir_all(parent).expect("cannot create directory");

    let mut f = fs::File::create(path).expect("cannot create CSV");
    writeln!(f, "timestamp_ms,rr_interval_ms,beat_type,bpm,mapped_freq").unwrap();

    let mut t = 0.0f32;

    let classes: [(f32, &str); 3] = [
        (820.0, "N"),  // Normal ~73 BPM
        (400.0, "N"),  // Tachy ~150 BPM (still N beats, just faster)
        (1500.0, "N"), // Brady ~40 BPM
    ];

    for &(rr, beat_type) in &classes {
        for _ in 0..n_per_class {
            let bpm = 60_000.0 / rr;
            let freq = 2000.0 + (bpm - 40.0) * (3000.0 / 160.0);
            writeln!(f, "{:.1},{:.1},{},{:.1},{:.1}", t, rr, beat_type, bpm, freq).unwrap();
            t += rr;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_sinus_bpm() {
        let c = normal_sinus();
        let bpm = c.bpm();
        assert!(bpm > 60.0 && bpm < 100.0, "Normal sinus BPM={bpm}");
    }

    #[test]
    fn tachycardia_bpm() {
        let c = tachycardia();
        assert!(c.bpm() > 100.0, "Tachy BPM={}", c.bpm());
    }

    #[test]
    fn bradycardia_bpm() {
        let c = bradycardia();
        assert!(c.bpm() < 60.0, "Brady BPM={}", c.bpm());
    }

    #[test]
    fn stream_length() {
        let c = normal_sinus();
        let stream = c.to_frequency_stream(3);
        assert_eq!(stream.len(), c.duration_ms() * 3);
    }

    #[test]
    fn csv_roundtrip() {
        let path = "/tmp/cricket_brain_test_sample.csv";
        write_sample_csv(path, 10);
        let records = from_csv(path);
        assert_eq!(records.len(), 30, "10 per class × 3 classes = 30");
        assert!(records[0].rr_interval_ms > 800.0); // Normal
        assert!(records[10].rr_interval_ms < 500.0); // Tachy
        assert!(records[20].rr_interval_ms > 1400.0); // Brady
                                                      // 5-col legacy CSV → record_id falls back to file stem
        assert_eq!(records[0].record_id, "cricket_brain_test_sample");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn csv_with_record_id_column() {
        // Synthetic 6-column CSV. Verifies that the v0.3 header is
        // detected and `record_id` is read from the row.
        let path = "/tmp/cricket_brain_test_record_id.csv";
        std::fs::write(
            path,
            "timestamp_ms,rr_interval_ms,beat_type,bpm,mapped_freq,record_id\n\
             0.0,820.0,N,73.2,2622.0,100\n\
             820.0,820.0,N,73.2,2622.0,100\n\
             1640.0,400.0,N,150.0,3000.0,200\n",
        )
        .unwrap();
        let records = from_csv(path);
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].record_id, "100");
        assert_eq!(records[2].record_id, "200");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn from_csv_dir_groups_by_record_id() {
        let dir = "/tmp/cricket_brain_test_csv_dir";
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            format!("{dir}/a.csv"),
            "timestamp_ms,rr_interval_ms,beat_type,bpm,mapped_freq,record_id\n\
             0.0,820.0,N,73.2,2622.0,100\n\
             820.0,820.0,N,73.2,2622.0,100\n",
        )
        .unwrap();
        std::fs::write(
            format!("{dir}/b.csv"),
            "timestamp_ms,rr_interval_ms,beat_type,bpm,mapped_freq,record_id\n\
             0.0,400.0,N,150.0,3000.0,200\n",
        )
        .unwrap();
        let groups = from_csv_dir(dir);
        assert_eq!(groups.len(), 2);
        let ids: Vec<&str> = groups.iter().map(|(id, _)| id.as_str()).collect();
        assert!(ids.contains(&"100"));
        assert!(ids.contains(&"200"));
        let beats_100 = groups.iter().find(|(id, _)| id == "100").unwrap();
        assert_eq!(beats_100.1.len(), 2);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn beats_to_stream() {
        let records = vec![BeatRecord {
            timestamp_ms: 0.0,
            rr_interval_ms: 820.0,
            beat_type: "N".to_string(),
            bpm: 73.0,
            mapped_freq: 2618.75,
            record_id: String::new(),
        }];
        let stream = beats_to_frequency_stream(&records);
        // 810 ms silence + 10 ms QRS = 820 total
        assert_eq!(stream.len(), 820);
        assert_eq!(stream[0], 0.0); // Silence
        assert_eq!(stream[819], 4500.0); // QRS
    }
}
