// SPDX-License-Identifier: AGPL-3.0-only
//! Synthetic 50 Hz grid signal generation for power-quality scenarios.
//!
//! Models the IEC 61000-4-30 power-quality categories that CricketBrain
//! can triage with a 4-channel resonator bank:
//!
//!   FUND  (Fundamental):       50 Hz
//!   H2    (2nd harmonic):     100 Hz   — DC offset, transformer saturation
//!   H3    (3rd harmonic):     150 Hz   — non-linear loads (rectifiers, VFDs)
//!   H4    (4th harmonic):     200 Hz   — switching artefacts, fast EMI
//!
//! These four frequencies sit on integer multiples of 50 Hz so the
//! `TokenVocabulary::new(&[...], 50.0, 200.0)` distribution lands
//! exactly on each tuned channel.

/// Characteristic power-grid frequencies (Hz, EU 50 Hz system).
pub const FUND_FREQ: f32 = 50.0;
pub const H2_FREQ: f32 = 100.0;
pub const H3_FREQ: f32 = 150.0;
pub const H4_FREQ: f32 = 200.0;

/// Healthy grid: 50 Hz fundamental dominates. Brief jitter ≈ ±0.05 Hz
/// (Western European grid Total Vector Error budget per IEC 60044-7).
pub fn nominal_grid(n_steps: usize) -> Vec<f32> {
    source_signal(n_steps, FUND_FREQ, 0xF050_0000, 30, 4)
}

/// Power outage: silence with rare low-amplitude transients (UPS chirps,
/// distant lightning, residual ringdown of a tripped transformer).
pub fn outage(n_steps: usize) -> Vec<f32> {
    let mut signal = Vec::with_capacity(n_steps);
    let mut rng_state: u64 = 0x0017_A6E0;
    for _ in 0..n_steps {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r = (rng_state >> 40) as f32 / (1u64 << 24) as f32;
        if r < 0.02 {
            // Rare 250-400 Hz transient (out of band)
            signal.push(250.0 + r * 3000.0);
        } else {
            signal.push(0.0);
        }
    }
    signal
}

/// 2nd harmonic dominant: 100 Hz. In a healthy grid the 2nd harmonic is
/// negligible (<1 % of fundamental). Strong 2nd-harmonic content is a
/// diagnostic for **DC offset**, **transformer in-rush saturation**, or
/// **half-wave-rectified loads**.
pub fn second_harmonic_dominant(n_steps: usize) -> Vec<f32> {
    source_signal(n_steps, H2_FREQ, 0x4002_0001, 25, 5)
}

/// 3rd harmonic dominant: 150 Hz. The single most common power-quality
/// issue. Caused by **non-linear loads**: variable-frequency drives,
/// switched-mode power supplies, LED ballasts, arc furnaces.
pub fn third_harmonic_dominant(n_steps: usize) -> Vec<f32> {
    source_signal(n_steps, H3_FREQ, 0x4003_0002, 25, 5)
}

/// 4th harmonic dominant: 200 Hz. Indicates **fast switching artefacts**
/// and EMI from high-frequency power-electronics (resonant inverters,
/// high-frequency wireless chargers). Less common in residential grids.
pub fn fourth_harmonic_dominant(n_steps: usize) -> Vec<f32> {
    source_signal(n_steps, H4_FREQ, 0x4004_0003, 25, 5)
}

/// Shared burst/gap generator with ±2 % frequency jitter (matching the
/// IEC 61000-4-30 Class A measurement uncertainty budget).
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
            let jitter = source_freq * 0.02 * (r * 2.0 - 1.0);
            signal.push(source_freq + jitter);
        } else {
            signal.push(0.0);
        }
    }
    signal
}

/// Simulate a **factory startup transient**: nominal 50 Hz, then a sudden
/// large variable-frequency-drive load adds 3rd-harmonic distortion for
/// `disturbance_steps`, then nominal returns. Mirrors the kind of event
/// IEEE 519 voltage-distortion limits target.
pub fn factory_startup(n_steps: usize, disturbance_steps: usize) -> Vec<f32> {
    let pre = (n_steps.saturating_sub(disturbance_steps)) / 2;
    let post = n_steps.saturating_sub(pre).saturating_sub(disturbance_steps);
    let mut sig = Vec::with_capacity(n_steps);
    sig.extend(nominal_grid(pre));
    sig.extend(third_harmonic_dominant(disturbance_steps));
    sig.extend(nominal_grid(post));
    sig
}

/// Simulate a **rolling brownout**: nominal grid with several brief
/// outage windows, mimicking automatic load-shedding cycles seen in
/// stressed transmission networks.
pub fn rolling_brownout(n_steps: usize, n_dips: usize, dip_len: usize) -> Vec<f32> {
    let mut sig = nominal_grid(n_steps);
    let dip_starts: Vec<usize> = (0..n_dips)
        .map(|k| (k + 1) * n_steps / (n_dips + 1))
        .collect();
    for start in dip_starts {
        let end = (start + dip_len).min(n_steps);
        for s in &mut sig[start..end] {
            *s = 0.0;
        }
    }
    sig
}

/// Simulate **fundamental + 3rd harmonic coexisting** — the typical
/// pattern measured at the secondary of a distribution transformer
/// feeding a building with mixed linear (motors) and non-linear (LED
/// lighting + computer PSUs) loads. Emits whichever component is on at
/// each step (50 Hz ~70 % of the time, 150 Hz ~30 %).
pub fn nominal_with_third_harmonic(n_steps: usize) -> Vec<f32> {
    let nom = nominal_grid(n_steps);
    let h3 = third_harmonic_dominant(n_steps);
    let mut out = Vec::with_capacity(n_steps);
    let mut rng_state: u64 = 0x4050_0150;
    for i in 0..n_steps {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r = (rng_state >> 40) as f32 / (1u64 << 24) as f32;
        // True 70/30 split with no fall-through bias: 70 % of steps draw
        // from the fundamental stream, 30 % from the H3 stream. If the
        // chosen source happens to be in its inter-burst gap, emit
        // silence rather than borrowing from the other stream.
        if r < 0.30 {
            out.push(h3[i]);
        } else {
            out.push(nom[i]);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// CSV I/O for preprocessed PMU windows
// ---------------------------------------------------------------------------

use std::fs;
use std::io::{BufRead, BufReader};

/// One preprocessed power-quality window read from CSV.
#[derive(Debug, Clone)]
pub struct GridWindow {
    pub timestamp_ms: f32,
    pub dominant_freq: f32,
    pub thd_pct: f32,
    pub event_label: String,
}

/// Read a preprocessed CSV file.
///
/// Expected columns: `timestamp_ms,dominant_freq,thd_pct,event_label`.
/// `thd_pct` is total harmonic distortion in percent (informative only;
/// the detector uses `dominant_freq`).
pub fn from_csv(path: &str) -> Vec<GridWindow> {
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
        records.push(GridWindow {
            timestamp_ms: cols[0].parse().unwrap_or(0.0),
            dominant_freq: cols[1].parse().unwrap_or(0.0),
            thd_pct: cols[2].parse().unwrap_or(0.0),
            event_label: cols[3].trim().to_string(),
        });
    }
    records
}

/// Convert grid windows to a dense per-step frequency stream.
pub fn windows_to_frequency_stream(windows: &[GridWindow], steps_per_window: usize) -> Vec<f32> {
    let mut stream = Vec::with_capacity(windows.len() * steps_per_window);
    for w in windows {
        stream.extend(std::iter::repeat(w.dominant_freq).take(steps_per_window));
    }
    stream
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn nominal_has_50hz() {
        let sig = nominal_grid(340);
        let count = sig.iter().filter(|&&f| near(f, FUND_FREQ, 2.0)).count();
        assert!(count > 220, "Expected dominant 50 Hz, got {count}/340");
    }

    #[test]
    fn outage_mostly_silent() {
        let sig = outage(1000);
        let active = sig.iter().filter(|&&f| f > 0.0).count();
        assert!(active < 50, "Outage must be mostly silent: {active}/1000 active");
    }

    #[test]
    fn h2_h3_h4_distinct() {
        let s2 = second_harmonic_dominant(200);
        let s3 = third_harmonic_dominant(200);
        let s4 = fourth_harmonic_dominant(200);
        let mean = |s: &[f32]| {
            let active: Vec<f32> = s.iter().filter(|&&f| f > 0.0).cloned().collect();
            active.iter().sum::<f32>() / active.len().max(1) as f32
        };
        assert!(near(mean(&s2), H2_FREQ, 5.0));
        assert!(near(mean(&s3), H3_FREQ, 5.0));
        assert!(near(mean(&s4), H4_FREQ, 5.0));
    }

    #[test]
    fn factory_startup_has_three_phases() {
        let sig = factory_startup(900, 300);
        // Middle 300 samples should be ~150 Hz dominant
        let mid: Vec<f32> = sig[300..600].iter().filter(|&&f| f > 0.0).cloned().collect();
        let mean_mid = mid.iter().sum::<f32>() / mid.len().max(1) as f32;
        assert!(near(mean_mid, H3_FREQ, 8.0), "Mid-section should be H3, got {mean_mid}");
        // Outer sections should be ~50 Hz
        let head: Vec<f32> = sig[..200].iter().filter(|&&f| f > 0.0).cloned().collect();
        let mean_head = head.iter().sum::<f32>() / head.len().max(1) as f32;
        assert!(near(mean_head, FUND_FREQ, 5.0), "Head-section should be fundamental, got {mean_head}");
    }

    #[test]
    fn rolling_brownout_has_dips() {
        let sig = rolling_brownout(1000, 3, 50);
        let silent_runs = sig
            .windows(40)
            .filter(|w| w.iter().all(|&f| f == 0.0))
            .count();
        assert!(silent_runs > 0, "Brownout must contain silent dip windows, got {silent_runs}");
    }

    #[test]
    fn nominal_with_third_harmonic_mixes_both() {
        let sig = nominal_with_third_harmonic(800);
        let nom = sig.iter().filter(|&&f| near(f, FUND_FREQ, 5.0)).count();
        let h3 = sig.iter().filter(|&&f| near(f, H3_FREQ, 5.0)).count();
        assert!(nom > 50, "Mixed signal must contain fundamental: {nom}");
        assert!(h3 > 30, "Mixed signal must contain 3rd harmonic: {h3}");
    }

    #[test]
    fn csv_read() {
        let records = from_csv("data/processed/sample_grid.csv");
        assert_eq!(records.len(), 200, "Expected 200 windows");
        assert_eq!(records[0].event_label, "Outage");
        assert_eq!(records[40].event_label, "Nominal");
        assert_eq!(records[80].event_label, "SecondHarmonic");
        assert_eq!(records[120].event_label, "ThirdHarmonic");
        assert_eq!(records[160].event_label, "FourthHarmonic");
    }
}
