// SPDX-License-Identifier: AGPL-3.0-only
//! Synthetic vibration signal generation for bearing fault simulation.
//!
//! Models SKF 6205-2RS bearing characteristic defect frequencies:
//! - BPFO (Ball Pass Frequency Outer): ~107 Hz
//! - BPFI (Ball Pass Frequency Inner): ~162 Hz
//! - BSF  (Ball Spin Frequency):       ~69 Hz
//! - FTF  (Fundamental Train Freq):    ~15 Hz
//!
//! Each fault type produces a dominant frequency that CricketBrain's
//! resonator bank can detect.

/// SKF 6205-2RS bearing characteristic frequencies (Hz).
pub const BPFO: f32 = 107.0;
pub const BPFI: f32 = 162.0;
pub const BSF: f32 = 69.0;
pub const FTF: f32 = 15.0;

/// Baseline vibration: low-energy broadband noise, no dominant fault frequency.
/// Simulated as silence with occasional random low-amplitude bursts.
pub fn normal_vibration(n_steps: usize) -> Vec<f32> {
    let mut signal = Vec::with_capacity(n_steps);
    let mut rng_state: u64 = 0xBEA0_1234;
    for _ in 0..n_steps {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r = (rng_state >> 40) as f32 / (1u64 << 24) as f32;
        // 5% chance of random out-of-band burst (simulates machine vibration)
        if r < 0.05 {
            let freq = 20.0 + r * 400.0; // Random 20-60 Hz (shaft speed harmonics)
            signal.push(freq);
        } else {
            signal.push(0.0);
        }
    }
    signal
}

/// Outer race fault: BPFO (107 Hz) dominant with periodic bursts.
/// In real bearings, outer race defects produce sharp impulses at BPFO rate.
pub fn outer_race_fault(n_steps: usize) -> Vec<f32> {
    fault_signal(n_steps, BPFO, 0xBF00_0001)
}

/// Inner race fault: BPFI (162 Hz) dominant.
pub fn inner_race_fault(n_steps: usize) -> Vec<f32> {
    fault_signal(n_steps, BPFI, 0xBF01_0002)
}

/// Ball defect: BSF (69 Hz) dominant.
pub fn ball_fault(n_steps: usize) -> Vec<f32> {
    fault_signal(n_steps, BSF, 0x0B5F_0003)
}

/// Generate a fault signal with the given dominant frequency.
/// Pattern: sustained fault frequency with brief gaps (simulating rotation).
fn fault_signal(n_steps: usize, fault_freq: f32, seed: u64) -> Vec<f32> {
    let mut signal = Vec::with_capacity(n_steps);
    let mut rng_state: u64 = seed;
    // Fault produces bursts of ~20 steps at fault frequency, then ~5 steps gap
    let burst_len = 20;
    let gap_len = 5;
    let cycle = burst_len + gap_len;

    for i in 0..n_steps {
        let in_burst = (i % cycle) < burst_len;
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r = (rng_state >> 40) as f32 / (1u64 << 24) as f32;

        if in_burst {
            // Fault frequency with slight jitter (±3%)
            let jitter = fault_freq * 0.03 * (r * 2.0 - 1.0);
            signal.push(fault_freq + jitter);
        } else {
            // Brief silence between fault impulse trains
            signal.push(0.0);
        }
    }
    signal
}

// ---------------------------------------------------------------------------
// CSV I/O for preprocessed bearing data
// ---------------------------------------------------------------------------

use std::fs;
use std::io::{BufRead, BufReader};

/// A single preprocessed frequency window from CSV.
#[derive(Debug, Clone)]
pub struct VibrationWindow {
    pub timestamp_ms: f32,
    pub dominant_freq: f32,
    pub amplitude: f32,
    pub fault_label: String,
}

/// Read preprocessed CSV (timestamp_ms,dominant_freq,amplitude,fault_label).
pub fn from_csv(path: &str) -> Vec<VibrationWindow> {
    let file = fs::File::open(path).expect("cannot open CSV");
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        let line = line.expect("cannot read line");
        if i == 0 { continue; } // Skip header
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() < 4 { continue; }
        records.push(VibrationWindow {
            timestamp_ms: cols[0].parse().unwrap_or(0.0),
            dominant_freq: cols[1].parse().unwrap_or(0.0),
            amplitude: cols[2].parse().unwrap_or(0.0),
            fault_label: cols[3].trim().to_string(),
        });
    }
    records
}

/// Convert vibration windows to a frequency stream for the BearingDetector.
/// Each window's dominant_freq is repeated for `steps_per_window` timesteps.
pub fn windows_to_frequency_stream(windows: &[VibrationWindow], steps_per_window: usize) -> Vec<f32> {
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
    fn normal_mostly_silent() {
        let sig = normal_vibration(1000);
        let active = sig.iter().filter(|&&f| f > 0.0).count();
        assert!(active < 100, "Normal should be mostly silent: {active}/1000 active");
    }

    #[test]
    fn outer_race_has_bpfo() {
        let sig = outer_race_fault(200);
        let bpfo_count = sig.iter().filter(|&&f| (f - BPFO).abs() < BPFO * 0.05).count();
        assert!(bpfo_count > 100, "Should have strong BPFO presence: {bpfo_count}/200");
    }

    #[test]
    fn inner_race_has_bpfi() {
        let sig = inner_race_fault(200);
        let bpfi_count = sig.iter().filter(|&&f| (f - BPFI).abs() < BPFI * 0.05).count();
        assert!(bpfi_count > 100, "Should have strong BPFI presence: {bpfi_count}/200");
    }

    #[test]
    fn ball_has_bsf() {
        let sig = ball_fault(200);
        let bsf_count = sig.iter().filter(|&&f| (f - BSF).abs() < BSF * 0.05).count();
        assert!(bsf_count > 100, "Should have strong BSF presence: {bsf_count}/200");
    }

    #[test]
    fn csv_read() {
        let records = from_csv("data/processed/sample_bearing.csv");
        assert_eq!(records.len(), 200, "Should have 200 windows");
        assert_eq!(records[0].fault_label, "Normal");
        assert_eq!(records[50].fault_label, "OR");
        assert_eq!(records[100].fault_label, "IR");
        assert_eq!(records[150].fault_label, "Ball");
    }

    #[test]
    fn fault_signals_different() {
        let outer = outer_race_fault(100);
        let inner = inner_race_fault(100);
        let ball = ball_fault(100);
        // Mean frequencies should differ
        let mean_f = |s: &[f32]| {
            let active: Vec<f32> = s.iter().filter(|&&f| f > 0.0).cloned().collect();
            active.iter().sum::<f32>() / active.len().max(1) as f32
        };
        let m_outer = mean_f(&outer);
        let m_inner = mean_f(&inner);
        let m_ball = mean_f(&ball);
        assert!((m_outer - BPFO).abs() < 10.0, "Outer mean={m_outer}");
        assert!((m_inner - BPFI).abs() < 10.0, "Inner mean={m_inner}");
        assert!((m_ball - BSF).abs() < 10.0, "Ball mean={m_ball}");
    }
}
