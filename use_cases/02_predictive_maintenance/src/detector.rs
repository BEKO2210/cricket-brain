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
}

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
            window_size: 50, // 50-step detection window
            window_step: 0,
            last_fault: FaultType::Normal,
            last_confidence: 0.0,
            step_count: 0,
        }
    }

    /// Feed one vibration frequency sample.
    /// Returns a fault classification at the end of each detection window.
    pub fn step(&mut self, input_freq: f32) -> Option<FaultType> {
        let outputs = self.bank.step(input_freq);
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
}
