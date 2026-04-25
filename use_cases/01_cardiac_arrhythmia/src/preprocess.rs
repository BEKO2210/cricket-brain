// SPDX-License-Identifier: AGPL-3.0-only
//! ECG signal preprocessing for noise rejection.
//!
//! Operates BEFORE CricketBrain — cleans the frequency stream to reject:
//! - Out-of-band noise (frequencies far from QRS carrier)
//! - Single-step in-band noise spikes (require temporal consistency)
//! - High-frequency frequency jitter (moving average smoothing)
//!
//! This module addresses the fundamental limitation that CricketBrain's
//! Gaussian tuning cannot distinguish real QRS from random in-band noise.
//! By requiring temporal consistency (a signal must persist for multiple
//! consecutive steps), single-step noise spikes are rejected while real
//! QRS complexes (which last ~10ms) pass through.

use std::collections::VecDeque;

/// Configurable preprocessing pipeline for ECG frequency streams.
#[derive(Debug, Clone)]
pub struct EcgPreprocessor {
    /// Center frequency of the QRS bandpass (Hz).
    band_center: f32,
    /// Allowed deviation from center (fraction, e.g. 0.15 = ±15%).
    band_width: f32,
    /// Minimum consecutive in-band steps to pass a signal through.
    /// A real QRS lasts ~10ms; noise is typically 1ms.
    min_consecutive: usize,
    /// Rolling window for moving average smoothing.
    window: VecDeque<f32>,
    /// Window size for smoothing.
    window_size: usize,
    /// Counter: how many recent steps have been in-band?
    consecutive_inband: usize,
    /// Tolerance: how many out-of-band steps are allowed within a burst.
    gap_tolerance: usize,
    /// Counter: steps since last in-band signal.
    steps_since_inband: usize,
    /// The frequency being tracked during a consecutive run.
    tracking_freq: f32,
}

impl EcgPreprocessor {
    /// Create a preprocessor tuned to the QRS carrier frequency.
    ///
    /// - `band_center`: QRS carrier (default 4500 Hz)
    /// - `band_width`: fractional tolerance (0.15 = ±15%)
    /// - `min_consecutive`: minimum steps for temporal consistency (default 3)
    /// - `window_size`: moving average window (default 3)
    pub fn new(
        band_center: f32,
        band_width: f32,
        min_consecutive: usize,
        window_size: usize,
    ) -> Self {
        Self {
            band_center,
            band_width,
            min_consecutive,
            window: VecDeque::with_capacity(window_size),
            window_size,
            consecutive_inband: 0,
            gap_tolerance: 2,
            steps_since_inband: 0,
            tracking_freq: 0.0,
        }
    }

    /// Default preprocessor for cardiac QRS detection.
    pub fn cardiac_default() -> Self {
        Self::new(
            4500.0, // QRS carrier
            0.15,   // ±15% band (wider than CricketBrain's ±10% to avoid clipping)
            3,      // Must be in-band for 3+ steps (within a tolerant window)
            3,      // 3-sample moving average
        )
    }

    /// Check if a frequency is within the allowed band.
    fn is_inband(&self, freq: f32) -> bool {
        if freq <= 0.0 {
            return false;
        }
        let deviation = (freq - self.band_center).abs() / self.band_center;
        deviation <= self.band_width
    }

    /// Process one frequency sample. Returns the cleaned frequency.
    ///
    /// - In-band signals that persist for `min_consecutive` steps pass through.
    /// - Single-step noise spikes are replaced with 0 (silence).
    /// - Out-of-band signals are always passed as-is (they won't trigger CricketBrain anyway).
    pub fn filter(&mut self, input_freq: f32) -> f32 {
        // Step 1: Bandpass check
        let inband = self.is_inband(input_freq);

        if inband {
            self.consecutive_inband += 1;
            self.steps_since_inband = 0;
            self.tracking_freq = input_freq;
        } else {
            self.steps_since_inband += 1;
            // Allow short gaps within a burst (noise spike in middle of QRS)
            if self.steps_since_inband > self.gap_tolerance {
                // Gap too long — this burst is over
                self.consecutive_inband = 0;
                self.tracking_freq = 0.0;
            }
            // else: keep consecutive_inband count alive through short gap
        }

        // Step 2: Temporal consistency gate
        let gated_freq = if inband && self.consecutive_inband >= self.min_consecutive {
            // Signal has been in-band long enough — pass through
            input_freq
        } else if !inband
            && self.consecutive_inband >= self.min_consecutive
            && self.steps_since_inband <= self.gap_tolerance
        {
            // Brief out-of-band gap within a validated burst — pass the tracking freq
            // (bridge the noise spike with the last known good frequency)
            self.tracking_freq
        } else if !inband && self.consecutive_inband < self.min_consecutive {
            // Out of band and no active burst — pass as-is
            input_freq
        } else {
            // In-band but too brief — suppress (likely noise spike)
            0.0
        };

        // Step 3: Moving average smoothing (only for in-band frequencies)
        if gated_freq > 0.0 {
            self.window.push_back(gated_freq);
            if self.window.len() > self.window_size {
                self.window.pop_front();
            }
            let avg = self.window.iter().sum::<f32>() / self.window.len() as f32;
            avg
        } else {
            // Clear smoothing window during silence
            self.window.clear();
            gated_freq
        }
    }

    /// Process an entire frequency stream. Returns cleaned stream.
    pub fn filter_stream(&mut self, input: &[f32]) -> Vec<f32> {
        self.reset();
        input.iter().map(|&f| self.filter(f)).collect()
    }

    /// Reset internal state.
    pub fn reset(&mut self) {
        self.window.clear();
        self.consecutive_inband = 0;
        self.steps_since_inband = 0;
        self.tracking_freq = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_sustained_qrs() {
        let mut pp = EcgPreprocessor::cardiac_default();
        // 10 consecutive steps at 4500 Hz — should pass after min_consecutive
        let input: Vec<f32> = vec![4500.0; 10];
        let output = pp.filter_stream(&input);
        // First 2 steps suppressed (min_consecutive=3), rest pass
        assert_eq!(output[0], 0.0, "Step 0 should be suppressed");
        assert_eq!(output[1], 0.0, "Step 1 should be suppressed");
        assert!(output[3] > 4000.0, "Step 3 should pass: {}", output[3]);
        assert!(output[9] > 4000.0, "Step 9 should pass: {}", output[9]);
    }

    #[test]
    fn rejects_single_noise_spike() {
        let mut pp = EcgPreprocessor::cardiac_default();
        // Pattern: silence, one noise spike at 4500, silence
        let input = vec![0.0, 0.0, 4500.0, 0.0, 0.0];
        let output = pp.filter_stream(&input);
        // The single in-band spike should be suppressed
        assert_eq!(output[2], 0.0, "Single spike should be rejected");
    }

    #[test]
    fn rejects_two_step_noise() {
        let mut pp = EcgPreprocessor::cardiac_default();
        // Two consecutive steps — still below min_consecutive=3
        let input = vec![0.0, 4500.0, 4500.0, 0.0, 0.0];
        let output = pp.filter_stream(&input);
        assert_eq!(output[1], 0.0, "2-step burst should be rejected");
        assert_eq!(output[2], 0.0, "2-step burst should be rejected");
    }

    #[test]
    fn passes_out_of_band_unchanged() {
        let mut pp = EcgPreprocessor::cardiac_default();
        // P wave at 3100 Hz — out of QRS band, should pass as-is
        let input = vec![3100.0, 3100.0, 3100.0];
        let output = pp.filter_stream(&input);
        assert_eq!(output[0], 3100.0);
        assert_eq!(output[2], 3100.0);
    }

    #[test]
    fn realistic_ecg_cycle() {
        let mut pp = EcgPreprocessor::cardiac_default();
        // P(3100,12) + gap(4) + QRS(4500,10) + gap(4) + T(3400,14)
        let mut input = Vec::new();
        input.extend(std::iter::repeat(3100.0f32).take(12)); // P wave
        input.extend(std::iter::repeat(0.0f32).take(4)); // gap
        input.extend(std::iter::repeat(4500.0f32).take(10)); // QRS
        input.extend(std::iter::repeat(0.0f32).take(4)); // gap
        input.extend(std::iter::repeat(3400.0f32).take(14)); // T wave

        let output = pp.filter_stream(&input);

        // P wave should pass (out of QRS band)
        assert!(output[0] > 3000.0, "P wave passes: {}", output[0]);

        // QRS should pass after 3 steps (steps 16-25 are QRS)
        let qrs_start = 16;
        assert_eq!(output[qrs_start], 0.0, "QRS step 0 suppressed");
        assert_eq!(output[qrs_start + 1], 0.0, "QRS step 1 suppressed");
        assert!(
            output[qrs_start + 3] > 4000.0,
            "QRS step 3 passes: {}",
            output[qrs_start + 3]
        );

        // T wave should pass (out of QRS band)
        assert!(output[30] > 3000.0, "T wave passes: {}", output[30]);
    }

    #[test]
    fn mixed_noise_and_signal() {
        let mut pp = EcgPreprocessor::cardiac_default();
        // 50 steps: mostly silence, with scattered noise spikes and one real QRS
        let mut input = vec![0.0f32; 50];
        // Noise spikes (single step)
        input[5] = 4600.0; // in-band noise
        input[12] = 4400.0; // in-band noise
        input[25] = 4500.0; // in-band noise
                            // Real QRS (sustained 10 steps)
        for i in 35..45 {
            input[i] = 4500.0;
        }

        let output = pp.filter_stream(&input);

        // All single noise spikes should be suppressed
        assert_eq!(output[5], 0.0, "Noise spike at 5 rejected");
        assert_eq!(output[12], 0.0, "Noise spike at 12 rejected");
        assert_eq!(output[25], 0.0, "Noise spike at 25 rejected");

        // Real QRS should pass (after 3 steps warm-up)
        assert!(
            output[38] > 4000.0,
            "Real QRS at step 38 passes: {}",
            output[38]
        );
        assert!(
            output[42] > 4000.0,
            "Real QRS at step 42 passes: {}",
            output[42]
        );
    }
}
