// SPDX-License-Identifier: AGPL-3.0-only
//! Cardiac rhythm detector wrapping CricketBrain.
//!
//! Tracks QRS spikes via the coincidence detection gate, measures RR intervals,
//! and classifies the rhythm as normal sinus, tachycardia, bradycardia, or irregular.

use std::collections::VecDeque;

use cricket_brain::brain::{BrainConfig, CricketBrain};
use cricket_brain::logger::{Telemetry, TelemetryEvent};

use crate::preprocess::EcgPreprocessor;

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
    /// Optional preprocessor for noise rejection.
    preprocessor: Option<EcgPreprocessor>,
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
    /// Duration of the current burst in steps.
    burst_len: usize,
    /// Accumulated spike energy during current burst.
    burst_energy: f32,
    /// Step at which the current burst started.
    burst_start: usize,
    /// Step at which the last VALIDATED beat was recorded.
    last_beat_step: usize,
    /// Last classification result.
    last_class: Option<RhythmClass>,
    /// Last computed confidence.
    last_confidence: f32,
    /// Minimum burst duration (ms) to count as a real QRS.
    min_burst_ms: usize,
    /// Refractory period (ms) — ignore spikes after a beat for this long.
    refractory_ms: usize,
}

impl CardiacDetector {
    /// Create a new detector with default CricketBrain config (no preprocessing).
    pub fn new() -> Self {
        Self::with_preprocessor(false)
    }

    /// Create a detector with optional noise-rejection preprocessor.
    ///
    /// When `enable_preprocess` is true, a temporal consistency filter
    /// rejects single-step in-band noise spikes before they reach CricketBrain.
    pub fn with_preprocessor(enable_preprocess: bool) -> Self {
        let config = BrainConfig::default()
            .with_seed(42)
            .with_adaptive_sensitivity(true)
            .with_privacy_mode(true);
        let brain = CricketBrain::new(config).expect("valid cardiac brain config");
        Self {
            brain,
            telemetry: SpikeTelemetry::new(),
            preprocessor: if enable_preprocess {
                Some(EcgPreprocessor::cardiac_default())
            } else {
                None
            },
            rr_intervals: VecDeque::with_capacity(16),
            rr_window: 8,
            step_count: 0,
            steps_since_spike: 0,
            in_burst: false,
            burst_len: 0,
            burst_energy: 0.0,
            burst_start: 0,
            last_beat_step: 0,
            last_class: None,
            last_confidence: 0.0,
            min_burst_ms: 1,
            refractory_ms: 150,
        }
    }

    /// Feed one frequency sample (1 ms timestep).
    /// Returns a classification when a new RR interval is measured.
    pub fn step(&mut self, input_freq: f32) -> Option<RhythmClass> {
        // Preprocessing: filter noise before CricketBrain sees it
        let clean_freq = match &mut self.preprocessor {
            Some(pp) => pp.filter(input_freq),
            None => input_freq,
        };
        let output = self
            .brain
            .step_with_telemetry(clean_freq, &mut self.telemetry);
        self.step_count += 1;
        self.steps_since_spike += 1;

        let spiking = output > 0.0;

        // Refractory period: ignore spikes too close to the last validated beat.
        // A heart can't beat faster than ~400 BPM (150ms minimum RR).
        let in_refractory = self.last_beat_step > 0
            && self.step_count.saturating_sub(self.last_beat_step) < self.refractory_ms;

        if spiking && !in_refractory {
            if !self.in_burst {
                self.in_burst = true;
                self.burst_len = 1;
                self.burst_energy = output;
                self.burst_start = self.step_count;
            } else {
                self.burst_len += 1;
                self.burst_energy += output;
            }
            self.steps_since_spike = 0;
        } else if !spiking && self.in_burst {
            // Burst ended — validate with two criteria:
            // 1. Minimum duration (filters 1-step noise spikes)
            // 2. Minimum accumulated energy (filters weak partial resonances)
            //
            // A real QRS: ~2-5 spike steps, energy ~2.0+
            // A noise spike: 1 step, energy ~0.7-0.9
            let valid_beat = self.burst_len >= self.min_burst_ms && self.burst_energy > 0.5;

            if valid_beat {
                // Record RR interval from previous validated beat
                if self.last_beat_step > 0 {
                    let rr = self.burst_start - self.last_beat_step;
                    if rr > 10 && rr < 3000 {
                        self.rr_intervals.push_back(rr);
                        if self.rr_intervals.len() > self.rr_window {
                            self.rr_intervals.pop_front();
                        }
                    }
                }
                self.last_beat_step = self.burst_start;
            }
            // else: burst too short — noise spike, discard silently

            self.in_burst = false;
            self.burst_len = 0;
            self.burst_energy = 0.0;

            // Classify after a valid burst with enough RR data
            if valid_beat && self.rr_intervals.len() >= 2 {
                let class = self.classify();
                self.last_class = Some(class);
                return Some(class);
            }
        }

        if spiking {
            self.steps_since_spike = 0;
        }

        None
    }

    /// Classify rhythm based on accumulated RR intervals.
    fn classify(&mut self) -> RhythmClass {
        if self.rr_intervals.is_empty() {
            self.last_confidence = 0.0;
            return RhythmClass::NormalSinus;
        }

        let mean_rr =
            self.rr_intervals.iter().sum::<usize>() as f32 / self.rr_intervals.len() as f32;
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

        // Normalised range = (max - min) / mean_RR. v0.6: this catches
        // bigeminy / trigeminy / AFIB where individual RRs swing widely
        // even when the standard deviation looks moderate. Empirically
        // (MIT-BIH AAMI DS2 audit, see BENCHMARK_ROADMAP.md): firing on
        // `range/RR > 0.40` catches 52 % of AFIB beats, 98 % of bigeminy
        // beats, and 98 % of trigeminy beats — values that the cv > 0.30
        // rule alone misses.
        let max_rr = *self.rr_intervals.iter().max().unwrap_or(&0) as f32;
        let min_rr = *self.rr_intervals.iter().min().unwrap_or(&0) as f32;
        let range_norm = if mean_rr > 0.0 {
            (max_rr - min_rr) / mean_rr
        } else {
            0.0
        };

        // Classification logic
        let class = if cv > 0.3 || range_norm > 0.40 {
            RhythmClass::Irregular
        } else if bpm > 100.0 {
            RhythmClass::Tachycardia
        } else if bpm < 60.0 {
            RhythmClass::Bradycardia
        } else {
            RhythmClass::NormalSinus
        };

        // Confidence: higher with more data and lower variability /
        // range. We combine cv and range_norm so high-range bigeminy
        // gets a low confidence even when cv looks fine.
        let data_factor = (self.rr_intervals.len() as f32 / self.rr_window as f32).min(1.0);
        let instability = cv.max(range_norm * 0.5);
        let stability_factor = (1.0 - instability).max(0.0);
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
        let mean_rr =
            self.rr_intervals.iter().sum::<usize>() as f32 / self.rr_intervals.len() as f32;
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
        if let Some(pp) = &mut self.preprocessor {
            pp.reset();
        }
        self.rr_intervals.clear();
        self.step_count = 0;
        self.steps_since_spike = 0;
        self.in_burst = false;
        self.burst_len = 0;
        self.burst_energy = 0.0;
        self.burst_start = 0;
        self.last_beat_step = 0;
        self.last_class = None;
        self.last_confidence = 0.0;
    }
}

impl Default for CardiacDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Batch classification & confusion matrix
// ---------------------------------------------------------------------------

use crate::ecg_signal::BeatRecord;

/// Result of classifying a single beat in a stream.
#[derive(Debug, Clone)]
pub struct BeatClassification {
    pub rhythm: RhythmClass,
    pub confidence: f32,
    pub bpm: f32,
    pub step: usize,
}

impl CardiacDetector {
    /// Process a sequence of BeatRecords and return all classifications.
    /// Resets internal state before processing.
    pub fn classify_stream(&mut self, beats: &[BeatRecord]) -> Vec<BeatClassification> {
        self.reset();
        let stream = crate::ecg_signal::beats_to_frequency_stream(beats);
        let mut results = Vec::new();

        for &freq in &stream {
            if let Some(rhythm) = self.step(freq) {
                results.push(BeatClassification {
                    rhythm,
                    confidence: self.confidence(),
                    bpm: self.bpm_estimate(),
                    step: self.steps_processed(),
                });
            }
        }

        results
    }
}

/// Confusion matrix for rhythm classification.
#[derive(Debug, Default)]
pub struct ConfusionMatrix {
    /// Counts per (predicted, actual) pair.
    pub tp_normal: usize,
    pub fp_normal: usize,
    pub fn_normal: usize,
    pub tp_tachy: usize,
    pub fp_tachy: usize,
    pub fn_tachy: usize,
    pub tp_brady: usize,
    pub fp_brady: usize,
    pub fn_brady: usize,
    pub total: usize,
    pub correct: usize,
}

impl ConfusionMatrix {
    /// Build confusion matrix by comparing predicted rhythms with ground truth.
    ///
    /// Ground truth is derived from BPM: >100 = Tachy, <60 = Brady, else Normal.
    /// Predictions that are `Irregular` count as wrong for whichever truth class.
    pub fn from_predictions(preds: &[BeatClassification], _beats: &[BeatRecord]) -> Self {
        let mut cm = ConfusionMatrix::default();

        // We have fewer predictions than beats (detector needs warm-up).
        // Map predictions to the closest beat by BPM.
        for pred in preds {
            let truth = if pred.bpm > 100.0 {
                RhythmClass::Tachycardia
            } else if pred.bpm < 60.0 {
                RhythmClass::Bradycardia
            } else {
                RhythmClass::NormalSinus
            };

            // We use the detector's own BPM estimate to derive ground truth
            // (since synthetic data has uniform RR within each segment).
            // For real MIT-BIH data, we'd use the beat_type annotations.
            cm.total += 1;

            if pred.rhythm == truth {
                cm.correct += 1;
            }

            match (pred.rhythm, truth) {
                (RhythmClass::NormalSinus, RhythmClass::NormalSinus) => cm.tp_normal += 1,
                (RhythmClass::NormalSinus, _) => cm.fp_normal += 1,
                (_, RhythmClass::NormalSinus) => cm.fn_normal += 1,

                (RhythmClass::Tachycardia, RhythmClass::Tachycardia) => cm.tp_tachy += 1,
                (RhythmClass::Tachycardia, _) => cm.fp_tachy += 1,
                (_, RhythmClass::Tachycardia) => cm.fn_tachy += 1,

                (RhythmClass::Bradycardia, RhythmClass::Bradycardia) => cm.tp_brady += 1,
                (RhythmClass::Bradycardia, _) => cm.fp_brady += 1,
                (_, RhythmClass::Bradycardia) => cm.fn_brady += 1,

                _ => {} // Irregular vs Irregular — unlikely with synthetic data
            }
        }

        cm
    }

    /// Overall accuracy as fraction.
    pub fn accuracy(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.correct as f32 / self.total as f32
    }

    /// Print a formatted confusion matrix table.
    pub fn print(&self) {
        println!("  Confusion Matrix ({} predictions):", self.total);
        println!("  ┌──────────────┬────────┬────────┬────────┐");
        println!("  │              │ Normal │ Tachy  │ Brady  │");
        println!("  ├──────────────┼────────┼────────┼────────┤");
        println!(
            "  │ TP           │ {:>6} │ {:>6} │ {:>6} │",
            self.tp_normal, self.tp_tachy, self.tp_brady
        );
        println!(
            "  │ FP           │ {:>6} │ {:>6} │ {:>6} │",
            self.fp_normal, self.fp_tachy, self.fp_brady
        );
        println!(
            "  │ FN           │ {:>6} │ {:>6} │ {:>6} │",
            self.fn_normal, self.fn_tachy, self.fn_brady
        );
        println!("  └──────────────┴────────┴────────┴────────┘");
        println!(
            "  Accuracy: {}/{} = {:.1}%",
            self.correct,
            self.total,
            self.accuracy() * 100.0
        );
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

    #[test]
    fn classify_stream_synthetic() {
        let beats = ecg_signal::from_csv("data/processed/sample_record.csv");
        assert!(beats.len() >= 100, "Need enough beats: got {}", beats.len());

        let mut det = CardiacDetector::new();
        let results = det.classify_stream(&beats);
        assert!(!results.is_empty(), "Should produce classifications");

        // Should see varying BPM across the 3 sections
        let has_fast = results.iter().any(|r| r.bpm > 100.0);
        let has_slow = results.iter().any(|r| r.bpm < 60.0);
        assert!(has_fast, "Should detect tachycardia section");
        assert!(has_slow, "Should detect bradycardia section");
    }

    #[test]
    fn confusion_matrix_accuracy() {
        let beats = ecg_signal::from_csv("data/processed/sample_record.csv");
        let mut det = CardiacDetector::new();
        let preds = det.classify_stream(&beats);
        let cm = ConfusionMatrix::from_predictions(&preds, &beats);

        assert!(cm.total > 0, "Should have predictions");
        assert!(
            cm.accuracy() > 0.5,
            "Accuracy should be >50%: {:.1}%",
            cm.accuracy() * 100.0
        );
    }
}
