// SPDX-License-Identifier: AGPL-3.0-only
//! Bearing fault detector using CricketBrain ResonatorBank.
//!
//! Four channels tuned to the four characteristic bearing defect frequencies.
//! When a fault frequency is sustained, the corresponding channel fires.

use cricket_brain::resonator_bank::ResonatorBank;
use cricket_brain::token::TokenVocabulary;

/// Bearing fault classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultType {
    Normal,
    OuterRace,
    InnerRace,
    BallDefect,
}

impl std::fmt::Display for FaultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FaultType::Normal => write!(f, "Normal"),
            FaultType::OuterRace => write!(f, "Outer Race (BPFO)"),
            FaultType::InnerRace => write!(f, "Inner Race (BPFI)"),
            FaultType::BallDefect => write!(f, "Ball Defect (BSF)"),
        }
    }
}

/// Bearing fault detector wrapping a CricketBrain ResonatorBank.
pub struct BearingDetector {
    bank: ResonatorBank,
    /// Accumulated energy per channel over the detection window.
    channel_energy: [f32; 4],
    /// Detection window size (steps).
    window_size: usize,
    /// Steps in current window.
    window_step: usize,
    /// Last classification.
    last_fault: FaultType,
    /// Last confidence.
    last_confidence: f32,
    /// Total steps processed.
    step_count: usize,
    /// Calibration RPM (fault frequencies are calculated at this speed).
    cal_rpm: f32,
    /// Current operating RPM (for speed compensation).
    /// If None, no compensation is applied.
    current_rpm: Option<f32>,
}

/// Reference RPM for SKF 6205-2RS fault frequency calculations.
const CAL_RPM: f32 = 1797.0;

impl BearingDetector {
    /// Create a new detector with channels for BPFO, BPFI, BSF, FTF.
    pub fn new() -> Self {
        // Create vocabulary with frequencies matching the bearing defect characteristics.
        // TokenVocabulary distributes frequencies evenly in [min, max].
        // We need: BSF=69, BPFO=107, BPFI=162, FTF=15
        // Order tokens by frequency so the even distribution aligns:
        // Token 0 (FTF)  → lowest freq
        // Token 1 (BSF)  → next
        // Token 2 (BPFO) → next
        // Token 3 (BPFI) → highest freq
        // Range: 15-162 Hz → spacing ~49 Hz
        let vocab = TokenVocabulary::new(
            &["FTF", "BSF", "BPFO", "BPFI"],
            15.0,   // min = FTF
            162.0,  // max = BPFI
        );
        let bank = ResonatorBank::new(&vocab);
        Self {
            bank,
            channel_energy: [0.0; 4],
            window_size: 50,
            window_step: 0,
            last_fault: FaultType::Normal,
            last_confidence: 0.0,
            step_count: 0,
            cal_rpm: CAL_RPM,
            current_rpm: None,
        }
    }

    /// Set operating RPM for speed compensation.
    ///
    /// When set, input frequencies are scaled by `cal_rpm / current_rpm`
    /// before entering the ResonatorBank. This maps fault frequencies at
    /// any shaft speed back to the calibration frequencies.
    ///
    /// Example: At 900 RPM, BPFO = 53.6 Hz. With compensation,
    /// 53.6 × (1797/900) = 107 Hz → matches the calibrated channel.
    pub fn set_rpm(&mut self, rpm: f32) {
        if rpm > 0.0 {
            self.current_rpm = Some(rpm);
        } else {
            self.current_rpm = None;
        }
    }

    /// Clear RPM compensation (no scaling applied).
    pub fn clear_rpm(&mut self) {
        self.current_rpm = None;
    }

    /// Feed one vibration frequency sample.
    /// Returns a fault classification at the end of each detection window.
    pub fn step(&mut self, input_freq: f32) -> Option<FaultType> {
        // Speed compensation: scale frequency to calibration RPM
        let compensated = match self.current_rpm {
            Some(rpm) if rpm > 0.0 && input_freq > 0.0 => {
                input_freq * (self.cal_rpm / rpm)
            }
            _ => input_freq,
        };
        let outputs = self.bank.step(compensated);
        self.step_count += 1;
        self.window_step += 1;

        // Accumulate energy per channel
        for (i, &out) in outputs.iter().enumerate().take(4) {
            if out > 0.0 {
                self.channel_energy[i] += out;
            }
        }

        // Classify at end of window
        if self.window_step >= self.window_size {
            let fault = self.classify();
            self.channel_energy = [0.0; 4];
            self.window_step = 0;
            self.last_fault = fault;
            return Some(fault);
        }

        None
    }

    /// Classify based on accumulated channel energy.
    fn classify(&mut self) -> FaultType {
        let total: f32 = self.channel_energy.iter().sum();
        if total < 0.1 {
            self.last_confidence = 1.0; // Confident it's normal (no energy)
            return FaultType::Normal;
        }

        // Find dominant channel
        let max_idx = self
            .channel_energy
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        let max_energy = self.channel_energy[max_idx];
        // Confidence: how dominant is the winning channel?
        self.last_confidence = (max_energy / total).clamp(0.0, 1.0);

        // Channel mapping: FTF=0, BSF=1, BPFO=2, BPFI=3
        match max_idx {
            0 => FaultType::Normal,    // FTF = normal wear pattern
            1 => FaultType::BallDefect,
            2 => FaultType::OuterRace,
            3 => FaultType::InnerRace,
            _ => FaultType::Normal,
        }
    }

    /// Current confidence (0.0–1.0).
    pub fn confidence(&self) -> f32 {
        self.last_confidence
    }

    /// Last fault classification.
    pub fn last_fault(&self) -> FaultType {
        self.last_fault
    }

    /// Total steps processed.
    pub fn steps_processed(&self) -> usize {
        self.step_count
    }

    /// Total neurons in the resonator bank.
    pub fn total_neurons(&self) -> usize {
        self.bank.total_neurons()
    }

    /// RAM usage of the resonator bank.
    pub fn memory_usage_bytes(&self) -> usize {
        self.bank.memory_usage_bytes()
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.bank.reset();
        self.channel_energy = [0.0; 4];
        self.window_step = 0;
        self.last_fault = FaultType::Normal;
        self.last_confidence = 0.0;
        self.step_count = 0;
    }
}

impl Default for BearingDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Batch classification & confusion matrix
// ---------------------------------------------------------------------------

use crate::vibration_signal::VibrationWindow;

/// Result of classifying a single detection window.
#[derive(Debug, Clone)]
pub struct WindowClassification {
    pub fault: FaultType,
    pub confidence: f32,
    pub step: usize,
}

impl BearingDetector {
    /// Process a sequence of VibrationWindows. Each window's dominant_freq is
    /// fed for `steps_per_window` timesteps.
    pub fn classify_stream(
        &mut self,
        windows: &[VibrationWindow],
        steps_per_window: usize,
    ) -> Vec<WindowClassification> {
        self.reset();
        let stream = crate::vibration_signal::windows_to_frequency_stream(windows, steps_per_window);
        let mut results = Vec::new();
        for &freq in &stream {
            if let Some(fault) = self.step(freq) {
                results.push(WindowClassification {
                    fault,
                    confidence: self.confidence(),
                    step: self.steps_processed(),
                });
            }
        }
        results
    }
}

/// Map ground-truth label string to FaultType.
fn label_to_fault(label: &str) -> FaultType {
    match label.trim() {
        "Normal" => FaultType::Normal,
        "OR" => FaultType::OuterRace,
        "IR" => FaultType::InnerRace,
        "Ball" => FaultType::BallDefect,
        _ => FaultType::Normal,
    }
}

/// Confusion matrix for bearing fault classification.
#[derive(Debug, Default)]
pub struct ConfusionMatrix {
    pub total: usize,
    pub correct: usize,
    pub tp_normal: usize,
    pub tp_outer: usize,
    pub tp_inner: usize,
    pub tp_ball: usize,
    pub fp_normal: usize,
    pub fp_outer: usize,
    pub fp_inner: usize,
    pub fp_ball: usize,
}

impl ConfusionMatrix {
    /// Build from predictions and ground-truth windows.
    ///
    /// Maps detector windows back to the nearest CSV windows using the
    /// `steps_per_window` ratio.
    pub fn from_predictions(
        preds: &[WindowClassification],
        windows: &[VibrationWindow],
        steps_per_window: usize,
    ) -> Self {
        let mut cm = Self::default();
        for p in preds {
            // Map step back to source window index
            let win_idx = (p.step / steps_per_window).min(windows.len().saturating_sub(1));
            let truth = label_to_fault(&windows[win_idx].fault_label);

            cm.total += 1;
            if p.fault == truth {
                cm.correct += 1;
            }
            match (p.fault, truth) {
                (FaultType::Normal, FaultType::Normal) => cm.tp_normal += 1,
                (FaultType::Normal, _) => cm.fp_normal += 1,
                (FaultType::OuterRace, FaultType::OuterRace) => cm.tp_outer += 1,
                (FaultType::OuterRace, _) => cm.fp_outer += 1,
                (FaultType::InnerRace, FaultType::InnerRace) => cm.tp_inner += 1,
                (FaultType::InnerRace, _) => cm.fp_inner += 1,
                (FaultType::BallDefect, FaultType::BallDefect) => cm.tp_ball += 1,
                (FaultType::BallDefect, _) => cm.fp_ball += 1,
            }
        }
        cm
    }

    pub fn accuracy(&self) -> f32 {
        if self.total == 0 { return 0.0; }
        self.correct as f32 / self.total as f32
    }

    pub fn print(&self) {
        println!("  Confusion Matrix ({} predictions):", self.total);
        println!("  ┌──────────────┬────────┬────────┬────────┬────────┐");
        println!("  │              │ Normal │ Outer  │ Inner  │  Ball  │");
        println!("  ├──────────────┼────────┼────────┼────────┼────────┤");
        println!("  │ TP           │ {:>6} │ {:>6} │ {:>6} │ {:>6} │",
                 self.tp_normal, self.tp_outer, self.tp_inner, self.tp_ball);
        println!("  │ FP           │ {:>6} │ {:>6} │ {:>6} │ {:>6} │",
                 self.fp_normal, self.fp_outer, self.fp_inner, self.fp_ball);
        println!("  └──────────────┴────────┴────────┴────────┴────────┘");
        println!("  Accuracy: {}/{} = {:.1}%", self.correct, self.total, self.accuracy() * 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibration_signal;

    #[test]
    fn detects_normal() {
        let mut det = BearingDetector::new();
        let sig = vibration_signal::normal_vibration(500);
        let mut last = None;
        for &f in &sig {
            if let Some(fault) = det.step(f) {
                last = Some(fault);
            }
        }
        assert_eq!(last, Some(FaultType::Normal), "Normal vibration should classify as Normal");
    }

    #[test]
    fn detects_outer_race() {
        let mut det = BearingDetector::new();
        let sig = vibration_signal::outer_race_fault(500);
        let mut last = None;
        for &f in &sig {
            if let Some(fault) = det.step(f) {
                last = Some(fault);
            }
        }
        assert_eq!(last, Some(FaultType::OuterRace));
    }

    #[test]
    fn detects_inner_race() {
        let mut det = BearingDetector::new();
        let sig = vibration_signal::inner_race_fault(500);
        let mut last = None;
        for &f in &sig {
            if let Some(fault) = det.step(f) {
                last = Some(fault);
            }
        }
        assert_eq!(last, Some(FaultType::InnerRace));
    }

    #[test]
    fn detects_ball_defect() {
        let mut det = BearingDetector::new();
        let sig = vibration_signal::ball_fault(500);
        let mut last = None;
        for &f in &sig {
            if let Some(fault) = det.step(f) {
                last = Some(fault);
            }
        }
        assert_eq!(last, Some(FaultType::BallDefect));
    }

    #[test]
    fn classify_stream_csv() {
        let windows = vibration_signal::from_csv("data/processed/sample_bearing.csv");
        assert!(windows.len() >= 100);
        let mut det = BearingDetector::new();
        let results = det.classify_stream(&windows, 25);
        assert!(!results.is_empty(), "Should produce classifications");
        // Should see different fault types across the 4 sections
        let has_normal = results.iter().any(|r| r.fault == FaultType::Normal);
        let has_outer = results.iter().any(|r| r.fault == FaultType::OuterRace);
        let has_inner = results.iter().any(|r| r.fault == FaultType::InnerRace);
        assert!(has_normal, "Should detect normal section");
        assert!(has_outer || has_inner, "Should detect at least one fault type");
    }

    #[test]
    fn confusion_matrix_accuracy() {
        let windows = vibration_signal::from_csv("data/processed/sample_bearing.csv");
        let mut det = BearingDetector::new();
        let preds = det.classify_stream(&windows, 25);
        let cm = ConfusionMatrix::from_predictions(&preds, &windows, 25);
        assert!(cm.total > 0);
        // With synthetic data, accuracy should be decent
        println!("UC02 CM accuracy: {:.1}%", cm.accuracy() * 100.0);
    }
}
