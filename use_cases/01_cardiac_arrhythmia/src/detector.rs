// SPDX-License-Identifier: AGPL-3.0-only
//! Cardiac rhythm detector wrapping CricketBrain.
//!
//! Tracks QRS spikes via the coincidence detection gate, measures RR intervals,
//! and classifies the rhythm as normal sinus, tachycardia, bradycardia, or irregular.

use std::collections::VecDeque;

use cricket_brain::brain::{BrainConfig, CricketBrain};
use cricket_brain::logger::{Telemetry, TelemetryEvent};

/// Rhythm classification output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmClass {
    NormalSinus,
    Tachycardia,
    Bradycardia,
    Irregular,
}

impl std::fmt::Display for RhythmClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RhythmClass::NormalSinus => write!(f, "Normal Sinus"),
            RhythmClass::Tachycardia => write!(f, "Tachycardia"),
            RhythmClass::Bradycardia => write!(f, "Bradycardia"),
            RhythmClass::Irregular => write!(f, "Irregular"),
        }
    }
}

/// Internal telemetry collector that tracks spike timestamps.
struct SpikeTelemetry {
    last_spike_step: Option<usize>,
    new_rr: Option<usize>,
}

impl SpikeTelemetry {
    fn new() -> Self {
        Self {
            last_spike_step: None,
            new_rr: None,
        }
    }
}

impl Telemetry for SpikeTelemetry {
    fn on_event(&mut self, event: TelemetryEvent) {
        if let TelemetryEvent::Spike { timestamp, .. } = event {
            let step = timestamp as usize;
            if let Some(prev) = self.last_spike_step {
                self.new_rr = Some(step.saturating_sub(prev));
            }
            self.last_spike_step = Some(step);
        }
    }
}

/// Cardiac arrhythmia detector using CricketBrain coincidence detection.
pub struct CardiacDetector {
    brain: CricketBrain,
    telemetry: SpikeTelemetry,
    /// Recent RR intervals (in ms/timesteps) for rhythm analysis.
    rr_intervals: VecDeque<usize>,
    /// How many RR intervals to keep for classification.
    rr_window: usize,
    /// Current timestep.
    step_count: usize,
    /// Steps since last QRS spike (for detecting gaps).
    steps_since_spike: usize,
    /// Whether we are currently in a spike burst.
    in_burst: bool,
    /// Step at which the current burst started.
    burst_start: usize,
    /// Last classification result.
    last_class: Option<RhythmClass>,
    /// Last computed confidence.
    last_confidence: f32,
}

impl CardiacDetector {
    /// Create a new detector with default CricketBrain config.
    pub fn new() -> Self {
        let config = BrainConfig::default()
            .with_seed(42)
            .with_adaptive_sensitivity(true)
            .with_privacy_mode(true);
        let brain = CricketBrain::new(config).expect("valid cardiac brain config");
        Self {
            brain,
            telemetry: SpikeTelemetry::new(),
            rr_intervals: VecDeque::with_capacity(16),
            rr_window: 8,
            step_count: 0,
            steps_since_spike: 0,
            in_burst: false,
            burst_start: 0,
            last_class: None,
            last_confidence: 0.0,
        }
    }

    /// Feed one frequency sample (1 ms timestep).
    /// Returns a classification when a new RR interval is measured.
    pub fn step(&mut self, input_freq: f32) -> Option<RhythmClass> {
        let output = self.brain.step_with_telemetry(input_freq, &mut self.telemetry);
        self.step_count += 1;
        self.steps_since_spike += 1;

        // Detect QRS burst boundaries
        let spiking = output > 0.0;
        if spiking && !self.in_burst {
            // Burst start — record RR interval from previous burst
            if self.burst_start > 0 {
                let rr = self.step_count - self.burst_start;
                if rr > 10 && rr < 3000 {
                    // Plausible RR: 20 BPM to 6000 BPM range
                    self.rr_intervals.push_back(rr);
                    if self.rr_intervals.len() > self.rr_window {
                        self.rr_intervals.pop_front();
                    }
                }
            }
            self.burst_start = self.step_count;
            self.in_burst = true;
            self.steps_since_spike = 0;
        } else if !spiking && self.in_burst {
            self.in_burst = false;
        }

        if spiking {
            self.steps_since_spike = 0;
        }

        // Classify when we have enough RR data and just finished a burst
        if !self.in_burst && self.steps_since_spike == 1 && self.rr_intervals.len() >= 2 {
            let class = self.classify();
            self.last_class = Some(class);
            return Some(class);
        }

        None
    }

    /// Classify rhythm based on accumulated RR intervals.
    fn classify(&mut self) -> RhythmClass {
        if self.rr_intervals.is_empty() {
            self.last_confidence = 0.0;
            return RhythmClass::NormalSinus;
        }

        let mean_rr = self.rr_intervals.iter().sum::<usize>() as f32
            / self.rr_intervals.len() as f32;
        let bpm = 60_000.0 / mean_rr;

        // Variability: coefficient of variation of RR intervals
        let variance = self
            .rr_intervals
            .iter()
            .map(|&rr| {
                let diff = rr as f32 - mean_rr;
                diff * diff
            })
            .sum::<f32>()
            / self.rr_intervals.len() as f32;
        let cv = variance.sqrt() / mean_rr;

        // Classification logic
        let class = if cv > 0.3 {
            RhythmClass::Irregular
        } else if bpm > 100.0 {
            RhythmClass::Tachycardia
        } else if bpm < 60.0 {
            RhythmClass::Bradycardia
        } else {
            RhythmClass::NormalSinus
        };

        // Confidence: higher with more data and lower variability
        let data_factor = (self.rr_intervals.len() as f32 / self.rr_window as f32).min(1.0);
        let stability_factor = (1.0 - cv).max(0.0);
        self.last_confidence = (data_factor * 0.4 + stability_factor * 0.6).clamp(0.0, 1.0);

        class
    }

    /// Current confidence (0.0 to 1.0).
    pub fn confidence(&self) -> f32 {
        self.last_confidence
    }

    /// Current BPM estimate from mean RR interval.
    pub fn bpm_estimate(&self) -> f32 {
        if self.rr_intervals.is_empty() {
            return 0.0;
        }
        let mean_rr = self.rr_intervals.iter().sum::<usize>() as f32
            / self.rr_intervals.len() as f32;
        60_000.0 / mean_rr
    }

    /// Last classification result.
    pub fn last_classification(&self) -> Option<RhythmClass> {
        self.last_class
    }

    /// Total timesteps processed.
    pub fn steps_processed(&self) -> usize {
        self.step_count
    }

    /// RAM usage of the underlying CricketBrain.
    pub fn memory_usage_bytes(&self) -> usize {
        self.brain.memory_usage_bytes()
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.brain.reset();
        self.telemetry = SpikeTelemetry::new();
        self.rr_intervals.clear();
        self.step_count = 0;
        self.steps_since_spike = 0;
        self.in_burst = false;
        self.burst_start = 0;
        self.last_class = None;
        self.last_confidence = 0.0;
    }
}

impl Default for CardiacDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecg_signal;

    #[test]
    fn detects_normal_sinus() {
        let mut det = CardiacDetector::new();
        // Need enough cycles to build up RR intervals
        let stream = ecg_signal::normal_sinus().to_frequency_stream(8);
        let mut classifications = Vec::new();
        for &freq in &stream {
            if let Some(class) = det.step(freq) {
                classifications.push(class);
            }
        }
        assert!(!classifications.is_empty(), "Should classify some beats");
        let bpm = det.bpm_estimate();
        assert!(bpm > 50.0 && bpm < 110.0, "Normal BPM={bpm}");
    }

    #[test]
    fn detects_tachycardia() {
        let mut det = CardiacDetector::new();
        let stream = ecg_signal::tachycardia().to_frequency_stream(10);
        let mut last = None;
        for &freq in &stream {
            if let Some(class) = det.step(freq) {
                last = Some(class);
            }
        }
        let bpm = det.bpm_estimate();
        assert!(bpm > 100.0, "Tachy BPM={bpm}");
        assert_eq!(last, Some(RhythmClass::Tachycardia));
    }

    #[test]
    fn detects_bradycardia() {
        let mut det = CardiacDetector::new();
        let stream = ecg_signal::bradycardia().to_frequency_stream(6);
        let mut last = None;
        for &freq in &stream {
            if let Some(class) = det.step(freq) {
                last = Some(class);
            }
        }
        let bpm = det.bpm_estimate();
        assert!(bpm < 60.0, "Brady BPM={bpm}");
        assert_eq!(last, Some(RhythmClass::Bradycardia));
    }
}
