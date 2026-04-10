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
}
