// SPDX-License-Identifier: AGPL-3.0-only
//! Synthetic hydrophone signal generation for marine acoustic scenarios.
//!
//! Models the characteristic frequencies of low-frequency ocean sounds:
//!   - FIN  (FinWhale 20-Hz pulse):           20 Hz
//!   - BLUE (Blue whale NE-Pacific A-call):   80 Hz
//!   - SHIP (Cargo-ship propeller cavitation): 140 Hz
//!   - HUMP (Humpback song mid-band):         200 Hz
//!
//! The MBARI MARS hydrophone samples the deep ocean off Monterey Bay at
//! 256 kHz. This module generates instantaneous-frequency streams suitable
//! for direct consumption by CricketBrain's `ResonatorBank`.

/// Characteristic marine-acoustic source frequencies (Hz).
///
/// These match the four evenly-spaced tokens emitted by
/// `TokenVocabulary::new(&[...], 20.0, 200.0)`.
pub const FIN_FREQ: f32 = 20.0;
pub const BLUE_FREQ: f32 = 80.0;
pub const SHIP_FREQ: f32 = 140.0;
pub const HUMP_FREQ: f32 = 200.0;

/// Ambient ocean noise: quiet baseline, occasional out-of-band burst.
///
/// Represents the quiescent deep-ocean soundscape. Most samples are silent
/// (below detection threshold). ~5% chance of a random low-amplitude burst
/// (biological noise, water movement, distant ship).
pub fn ambient_noise(n_steps: usize) -> Vec<f32> {
    let mut signal = Vec::with_capacity(n_steps);
    let mut rng_state: u64 = 0x0CEA_0000;
    for _ in 0..n_steps {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r = (rng_state >> 40) as f32 / (1u64 << 24) as f32;
        if r < 0.05 {
            // Random out-of-band burst 250-400 Hz (biological / water motion)
            let freq = 250.0 + r * 3000.0;
            signal.push(freq);
        } else {
            signal.push(0.0);
        }
    }
    signal
}

/// Fin whale 20-Hz stereotyped pulse train.
///
/// Real fin whales emit ~1-second 20-Hz pulses every 10-30 seconds. We
/// compress the inter-pulse interval to keep the signal dense enough for
/// the detector's 50-step energy window.
pub fn fin_whale_call(n_steps: usize) -> Vec<f32> {
    source_signal(n_steps, FIN_FREQ, 0xF1_0020, 22, 6)
}

/// Blue whale A-call: 80 Hz tonal downswept moan.
///
/// A-calls are longer (~20 s in real whales). We model them as a sustained
/// 80 Hz signal with brief silences.
pub fn blue_whale_call(n_steps: usize) -> Vec<f32> {
    source_signal(n_steps, BLUE_FREQ, 0xB1_0080, 30, 4)
}

/// Ship passage: sustained 140 Hz cargo-ship propeller cavitation.
///
/// A merchant vessel transiting past the hydrophone emits radiated noise
/// with a broad spectral peak around 100-200 Hz. The signal persists for
/// minutes, not seconds, hence the very long burst / short gap pattern.
pub fn ship_passage(n_steps: usize) -> Vec<f32> {
    source_signal(n_steps, SHIP_FREQ, 0x5417_0140, 40, 2)
}

/// Humpback whale song unit: 200 Hz mid-band note.
///
/// Humpback song is highly structured; each unit lasts a few seconds with
/// brief silences between units.
pub fn humpback_song(n_steps: usize) -> Vec<f32> {
    source_signal(n_steps, HUMP_FREQ, 0x4037_0200, 25, 5)
}

/// Shared burst/gap generator with ±3% frequency jitter.
fn source_signal(
    n_steps: usize,
    source_freq: f32,
    seed: u64,
    burst_len: usize,
    gap_len: usize,
) -> Vec<f32> {
    let mut signal = Vec::with_capacity(n_steps);
    let mut rng_state: u64 = seed;
    let cycle = burst_len + gap_len;

    for i in 0..n_steps {
        let in_burst = (i % cycle) < burst_len;
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r = (rng_state >> 40) as f32 / (1u64 << 24) as f32;

        if in_burst {
            let jitter = source_freq * 0.03 * (r * 2.0 - 1.0);
            signal.push(source_freq + jitter);
        } else {
            signal.push(0.0);
        }
    }
    signal
}

/// Simulate a ship sailing past the hydrophone over `n_steps`.
///
/// Amplitude (presence) rises as the ship approaches, plateaus at closest
/// point of approach (CPA), then falls as it recedes. Used to stress-test
/// the detector on a realistic vessel transit profile.
///
/// The signal returned still encodes frequency (Hz) per step, but when
/// the ship is "distant" the signal is silent (below noise floor). When
/// closer, 140 Hz is present more of the time.
pub fn ship_transit(n_steps: usize) -> Vec<f32> {
    let mut signal = Vec::with_capacity(n_steps);
    let mut rng_state: u64 = 0x5417_5A1;
    let mid = (n_steps / 2) as f32;

    for i in 0..n_steps {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r = (rng_state >> 40) as f32 / (1u64 << 24) as f32;

        // Triangular presence profile: 0% presence at edges, 100% at CPA.
        let dist = (i as f32 - mid).abs() / mid; // 1.0 at edges, 0.0 at CPA
        let presence = (1.0 - dist).clamp(0.0, 1.0);

        if r < presence {
            let jitter = SHIP_FREQ * 0.03 * (r * 2.0 - 1.0);
            signal.push(SHIP_FREQ + jitter);
        } else {
            signal.push(0.0);
        }
    }
    signal
}

/// Overlay a whale call on a ship passage — realistic real-ocean scenario
/// where endangered species vocalize through anthropogenic noise.
///
/// The fin whale pulses appear at ~25% of steps, masked by the steady ship
/// noise at 140 Hz on the remaining ~75% of steps.
pub fn fin_whale_under_ship(n_steps: usize) -> Vec<f32> {
    let fin = fin_whale_call(n_steps);
    let ship = ship_passage(n_steps);
    let mut out = Vec::with_capacity(n_steps);
    let mut rng_state: u64 = 0xF15417;

    for i in 0..n_steps {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r = (rng_state >> 40) as f32 / (1u64 << 24) as f32;
        // 25% of the time, prefer the fin-whale pulse if it is active;
        // otherwise prefer the ship signal.
        if r < 0.25 && fin[i] > 0.0 {
            out.push(fin[i]);
        } else if ship[i] > 0.0 {
            out.push(ship[i]);
        } else {
            out.push(fin[i]); // fall-through to whatever is present
        }
    }
    out
}

// ---------------------------------------------------------------------------
// CSV I/O for preprocessed hydrophone windows
// ---------------------------------------------------------------------------

use std::fs;
use std::io::{BufRead, BufReader};

/// One preprocessed acoustic window read from CSV.
#[derive(Debug, Clone)]
pub struct AcousticWindow {
    pub timestamp_ms: f32,
    pub dominant_freq: f32,
    pub rms_db: f32,
    pub event_label: String,
}

/// Read a preprocessed CSV file.
///
/// Expected columns: `timestamp_ms,dominant_freq,rms_db,event_label`.
/// Unknown / missing values become 0.0.
pub fn from_csv(path: &str) -> Vec<AcousticWindow> {
    let file = fs::File::open(path).expect("cannot open CSV");
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        let line = line.expect("cannot read line");
        if i == 0 {
            continue;
        }
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() < 4 {
            continue;
        }
        records.push(AcousticWindow {
            timestamp_ms: cols[0].parse().unwrap_or(0.0),
            dominant_freq: cols[1].parse().unwrap_or(0.0),
            rms_db: cols[2].parse().unwrap_or(0.0),
            event_label: cols[3].trim().to_string(),
        });
    }
    records
}

/// Convert acoustic windows to a dense per-step frequency stream.
///
/// Each window's dominant frequency is repeated for `steps_per_window` steps.
pub fn windows_to_frequency_stream(
    windows: &[AcousticWindow],
    steps_per_window: usize,
) -> Vec<f32> {
    let mut stream = Vec::with_capacity(windows.len() * steps_per_window);
    for w in windows {
        stream.extend(std::iter::repeat(w.dominant_freq).take(steps_per_window));
    }
    stream
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambient_mostly_silent() {
        let sig = ambient_noise(1000);
        let active = sig.iter().filter(|&&f| f > 0.0).count();
        assert!(active < 100, "Ambient should be mostly silent: {active}/1000 active");
    }

    #[test]
    fn fin_whale_has_20_hz() {
        let sig = fin_whale_call(280);
        let count = sig
            .iter()
            .filter(|&&f| (f - FIN_FREQ).abs() < FIN_FREQ * 0.1)
            .count();
        assert!(count > 150, "Fin whale pulses missing: {count}/280");
    }

    #[test]
    fn blue_whale_has_80_hz() {
        let sig = blue_whale_call(340);
        let count = sig
            .iter()
            .filter(|&&f| (f - BLUE_FREQ).abs() < BLUE_FREQ * 0.05)
            .count();
        assert!(count > 200, "Blue whale A-call missing: {count}/340");
    }

    #[test]
    fn ship_has_140_hz() {
        let sig = ship_passage(420);
        let count = sig
            .iter()
            .filter(|&&f| (f - SHIP_FREQ).abs() < SHIP_FREQ * 0.05)
            .count();
        assert!(count > 300, "Ship cavitation missing: {count}/420");
    }

    #[test]
    fn humpback_has_200_hz() {
        let sig = humpback_song(300);
        let count = sig
            .iter()
            .filter(|&&f| (f - HUMP_FREQ).abs() < HUMP_FREQ * 0.05)
            .count();
        assert!(count > 180, "Humpback song missing: {count}/300");
    }

    #[test]
    fn ship_transit_peaks_at_cpa() {
        let sig = ship_transit(1000);
        // First and last 10% of the signal should be mostly silent.
        let edge_active = sig[..100]
            .iter()
            .chain(sig[900..].iter())
            .filter(|&&f| f > 0.0)
            .count();
        let middle_active = sig[450..550].iter().filter(|&&f| f > 0.0).count();
        assert!(
            middle_active > edge_active * 4,
            "CPA should dominate: middle={middle_active} edges={edge_active}"
        );
    }

    #[test]
    fn fin_whale_under_ship_contains_both() {
        let sig = fin_whale_under_ship(600);
        let fin_count = sig
            .iter()
            .filter(|&&f| (f - FIN_FREQ).abs() < 5.0)
            .count();
        let ship_count = sig
            .iter()
            .filter(|&&f| (f - SHIP_FREQ).abs() < 10.0)
            .count();
        assert!(fin_count > 30, "Fin whale pulses must survive mixing: {fin_count}");
        assert!(ship_count > 100, "Ship noise must dominate most of the signal: {ship_count}");
    }

    #[test]
    fn signal_sources_distinct() {
        let mean_f = |s: &[f32]| {
            let active: Vec<f32> = s.iter().filter(|&&f| f > 0.0).cloned().collect();
            active.iter().sum::<f32>() / active.len().max(1) as f32
        };
        assert!((mean_f(&fin_whale_call(200)) - FIN_FREQ).abs() < 5.0);
        assert!((mean_f(&blue_whale_call(200)) - BLUE_FREQ).abs() < 5.0);
        assert!((mean_f(&ship_passage(200)) - SHIP_FREQ).abs() < 10.0);
        assert!((mean_f(&humpback_song(200)) - HUMP_FREQ).abs() < 10.0);
    }

    #[test]
    fn csv_read() {
        let records = from_csv("data/processed/sample_marine.csv");
        assert_eq!(records.len(), 200, "Should have 200 windows");
        assert_eq!(records[0].event_label, "Ambient");
        assert_eq!(records[40].event_label, "FinWhale");
        assert_eq!(records[80].event_label, "BlueWhale");
        assert_eq!(records[120].event_label, "Ship");
        assert_eq!(records[160].event_label, "Humpback");
    }
}
