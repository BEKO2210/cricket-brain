// SPDX-License-Identifier: AGPL-3.0-only
//! Marine acoustic classifier using a 4-channel CricketBrain `ResonatorBank`.
//!
//! Input: instantaneous dominant frequency (Hz) from a hydrophone window.
//! Output: one of five `AcousticEvent` values at the end of each detection
//! window (50 steps by default).

use cricket_brain::resonator_bank::ResonatorBank;
use cricket_brain::token::TokenVocabulary;

/// Marine acoustic event categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcousticEvent {
    /// No strong signal in any channel — quiescent ocean.
    Ambient,
    /// Fin whale 20-Hz stereotyped pulse (Balaenoptera physalus).
    FinWhale,
    /// Blue whale A-call, ~80 Hz tonal moan (Balaenoptera musculus).
    BlueWhale,
    /// Ship propeller cavitation and radiated noise peak (~140 Hz).
    ShipNoise,
    /// Humpback whale song unit (~200 Hz; Megaptera novaeangliae).
    Humpback,
}

impl std::fmt::Display for AcousticEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcousticEvent::Ambient => write!(f, "Ambient"),
            AcousticEvent::FinWhale => write!(f, "Fin Whale (20 Hz)"),
            AcousticEvent::BlueWhale => write!(f, "Blue Whale (80 Hz)"),
            AcousticEvent::ShipNoise => write!(f, "Ship Noise (140 Hz)"),
            AcousticEvent::Humpback => write!(f, "Humpback Song (200 Hz)"),
        }
    }
}

/// 4-channel marine acoustic classifier.
pub struct MarineDetector {
    bank: ResonatorBank,
    channel_energy: [f32; 4],
    window_size: usize,
    window_step: usize,
    last_event: AcousticEvent,
    last_confidence: f32,
    step_count: usize,
    /// Minimum total-channel energy below which we declare `Ambient`.
    /// Raised when the sea is noisy (set via `set_sea_state`).
    ambient_threshold: f32,
}

/// Default ambient threshold tuned for a calm-sea hydrophone.
const DEFAULT_AMBIENT_THRESHOLD: f32 = 0.1;

impl MarineDetector {
    /// Create a new detector with channels for FIN, BLUE, SHIP, HUMP.
    ///
    /// `TokenVocabulary` distributes frequencies evenly in `[min, max]`.
    /// Order the token labels from lowest to highest so each one lands on
    /// its characteristic marine-acoustic frequency:
    ///
    ///   Token 0 (FIN)  →  20 Hz
    ///   Token 1 (BLUE) →  80 Hz
    ///   Token 2 (SHIP) → 140 Hz
    ///   Token 3 (HUMP) → 200 Hz
    pub fn new() -> Self {
        let vocab = TokenVocabulary::new(
            &["FIN", "BLUE", "SHIP", "HUMP"],
            20.0,  // min = fin whale 20-Hz pulse
            200.0, // max = humpback song mid-band
        );
        let bank = ResonatorBank::new(&vocab);
        Self {
            bank,
            channel_energy: [0.0; 4],
            window_size: 50,
            window_step: 0,
            last_event: AcousticEvent::Ambient,
            last_confidence: 0.0,
            step_count: 0,
            ambient_threshold: DEFAULT_AMBIENT_THRESHOLD,
        }
    }

    /// Set the ambient-noise threshold explicitly (advanced use).
    pub fn set_ambient_threshold(&mut self, threshold: f32) {
        self.ambient_threshold = threshold.max(0.0);
    }

    /// Convenience: tune the ambient threshold to a Douglas sea state (0-9).
    ///
    /// Sea state 0 is mirror-calm; state 9 is phenomenal. Each step above
    /// state 0 raises the ambient threshold by 25 %. A loud storm should
    /// not trigger false-positive whale detections.
    pub fn set_sea_state(&mut self, state: u8) {
        let s = state.min(9) as f32;
        self.ambient_threshold = DEFAULT_AMBIENT_THRESHOLD * (1.0 + 0.25 * s);
    }

    /// Feed one dominant-frequency sample from the hydrophone.
    /// Returns a classification at the end of each detection window.
    pub fn step(&mut self, input_freq: f32) -> Option<AcousticEvent> {
        let outputs = self.bank.step(input_freq);
        self.step_count += 1;
        self.window_step += 1;

        for (i, &out) in outputs.iter().enumerate().take(4) {
            if out > 0.0 {
                self.channel_energy[i] += out;
            }
        }

        if self.window_step >= self.window_size {
            let event = self.classify();
            self.channel_energy = [0.0; 4];
            self.window_step = 0;
            self.last_event = event;
            return Some(event);
        }
        None
    }

    fn classify(&mut self) -> AcousticEvent {
        let total: f32 = self.channel_energy.iter().sum();
        if total < self.ambient_threshold {
            self.last_confidence = 1.0;
            return AcousticEvent::Ambient;
        }

        let max_idx = self
            .channel_energy
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        let max_energy = self.channel_energy[max_idx];
        self.last_confidence = (max_energy / total).clamp(0.0, 1.0);

        // Token ordering: FIN=0, BLUE=1, SHIP=2, HUMP=3.
        match max_idx {
            0 => AcousticEvent::FinWhale,
            1 => AcousticEvent::BlueWhale,
            2 => AcousticEvent::ShipNoise,
            3 => AcousticEvent::Humpback,
            _ => AcousticEvent::Ambient,
        }
    }

    pub fn confidence(&self) -> f32 {
        self.last_confidence
    }

    pub fn last_event(&self) -> AcousticEvent {
        self.last_event
    }

    pub fn steps_processed(&self) -> usize {
        self.step_count
    }

    pub fn total_neurons(&self) -> usize {
        self.bank.total_neurons()
    }

    pub fn memory_usage_bytes(&self) -> usize {
        self.bank.memory_usage_bytes()
    }

    pub fn reset(&mut self) {
        self.bank.reset();
        self.channel_energy = [0.0; 4];
        self.window_step = 0;
        self.last_event = AcousticEvent::Ambient;
        self.last_confidence = 0.0;
        self.step_count = 0;
    }
}

impl Default for MarineDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Batch classification & confusion matrix
// ---------------------------------------------------------------------------

use crate::acoustic_signal::AcousticWindow;

/// One detector decision, keyed by the step at which it was produced.
#[derive(Debug, Clone)]
pub struct WindowClassification {
    pub event: AcousticEvent,
    pub confidence: f32,
    pub step: usize,
}

impl MarineDetector {
    /// Process a sequence of preprocessed hydrophone windows. Each window's
    /// dominant frequency is fed for `steps_per_window` steps.
    pub fn classify_stream(
        &mut self,
        windows: &[AcousticWindow],
        steps_per_window: usize,
    ) -> Vec<WindowClassification> {
        self.reset();
        let stream = crate::acoustic_signal::windows_to_frequency_stream(windows, steps_per_window);
        let mut results = Vec::new();
        for &freq in &stream {
            if let Some(event) = self.step(freq) {
                results.push(WindowClassification {
                    event,
                    confidence: self.confidence(),
                    step: self.steps_processed(),
                });
            }
        }
        results
    }
}

/// Map a CSV ground-truth label to an `AcousticEvent`.
fn label_to_event(label: &str) -> AcousticEvent {
    match label.trim() {
        "Ambient" => AcousticEvent::Ambient,
        "FinWhale" => AcousticEvent::FinWhale,
        "BlueWhale" => AcousticEvent::BlueWhale,
        "Ship" | "ShipNoise" => AcousticEvent::ShipNoise,
        "Humpback" => AcousticEvent::Humpback,
        _ => AcousticEvent::Ambient,
    }
}

/// 5-class confusion matrix for marine acoustic events.
#[derive(Debug, Default)]
pub struct ConfusionMatrix {
    pub total: usize,
    pub correct: usize,
    pub tp_ambient: usize,
    pub tp_fin: usize,
    pub tp_blue: usize,
    pub tp_ship: usize,
    pub tp_hump: usize,
    pub fp_ambient: usize,
    pub fp_fin: usize,
    pub fp_blue: usize,
    pub fp_ship: usize,
    pub fp_hump: usize,
}

impl ConfusionMatrix {
    pub fn from_predictions(
        preds: &[WindowClassification],
        windows: &[AcousticWindow],
        steps_per_window: usize,
    ) -> Self {
        let mut cm = Self::default();
        for p in preds {
            let win_idx = (p.step / steps_per_window).min(windows.len().saturating_sub(1));
            let truth = label_to_event(&windows[win_idx].event_label);

            cm.total += 1;
            if p.event == truth {
                cm.correct += 1;
            }
            match (p.event, truth) {
                (AcousticEvent::Ambient, AcousticEvent::Ambient) => cm.tp_ambient += 1,
                (AcousticEvent::Ambient, _) => cm.fp_ambient += 1,
                (AcousticEvent::FinWhale, AcousticEvent::FinWhale) => cm.tp_fin += 1,
                (AcousticEvent::FinWhale, _) => cm.fp_fin += 1,
                (AcousticEvent::BlueWhale, AcousticEvent::BlueWhale) => cm.tp_blue += 1,
                (AcousticEvent::BlueWhale, _) => cm.fp_blue += 1,
                (AcousticEvent::ShipNoise, AcousticEvent::ShipNoise) => cm.tp_ship += 1,
                (AcousticEvent::ShipNoise, _) => cm.fp_ship += 1,
                (AcousticEvent::Humpback, AcousticEvent::Humpback) => cm.tp_hump += 1,
                (AcousticEvent::Humpback, _) => cm.fp_hump += 1,
            }
        }
        cm
    }

    pub fn accuracy(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.correct as f32 / self.total as f32
    }

    pub fn print(&self) {
        println!("  Confusion Matrix ({} predictions):", self.total);
        println!("  ┌──────────────┬────────┬────────┬────────┬────────┬────────┐");
        println!("  │              │ Ambien │  Fin   │  Blue  │  Ship  │  Hump  │");
        println!("  ├──────────────┼────────┼────────┼────────┼────────┼────────┤");
        println!(
            "  │ TP           │ {:>6} │ {:>6} │ {:>6} │ {:>6} │ {:>6} │",
            self.tp_ambient, self.tp_fin, self.tp_blue, self.tp_ship, self.tp_hump
        );
        println!(
            "  │ FP           │ {:>6} │ {:>6} │ {:>6} │ {:>6} │ {:>6} │",
            self.fp_ambient, self.fp_fin, self.fp_blue, self.fp_ship, self.fp_hump
        );
        println!("  └──────────────┴────────┴────────┴────────┴────────┴────────┘");
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
    use crate::acoustic_signal;

    fn last_event(sig: &[f32]) -> Option<AcousticEvent> {
        let mut det = MarineDetector::new();
        let mut last = None;
        for &f in sig {
            if let Some(e) = det.step(f) {
                last = Some(e);
            }
        }
        last
    }

    #[test]
    fn detects_ambient() {
        let sig = acoustic_signal::ambient_noise(500);
        assert_eq!(last_event(&sig), Some(AcousticEvent::Ambient));
    }

    #[test]
    fn detects_fin_whale() {
        let sig = acoustic_signal::fin_whale_call(500);
        assert_eq!(last_event(&sig), Some(AcousticEvent::FinWhale));
    }

    #[test]
    fn detects_blue_whale() {
        let sig = acoustic_signal::blue_whale_call(500);
        assert_eq!(last_event(&sig), Some(AcousticEvent::BlueWhale));
    }

    #[test]
    fn detects_ship() {
        let sig = acoustic_signal::ship_passage(500);
        assert_eq!(last_event(&sig), Some(AcousticEvent::ShipNoise));
    }

    #[test]
    fn detects_humpback() {
        let sig = acoustic_signal::humpback_song(500);
        assert_eq!(last_event(&sig), Some(AcousticEvent::Humpback));
    }

    #[test]
    fn detects_ship_transit_at_closest_approach() {
        // Simulate a cargo ship sailing past the hydrophone.
        // The classifier should flag ShipNoise during the middle of the
        // transit (closest point of approach).
        let sig = acoustic_signal::ship_transit(2000);
        let mut det = MarineDetector::new();
        let mut ship_hits = 0usize;
        let mut total_decisions = 0usize;
        for &f in &sig {
            if let Some(ev) = det.step(f) {
                total_decisions += 1;
                if ev == AcousticEvent::ShipNoise {
                    ship_hits += 1;
                }
            }
        }
        assert!(total_decisions > 0);
        assert!(
            ship_hits >= total_decisions / 3,
            "Ship should dominate at least ~1/3 of windows during transit \
             (got {ship_hits}/{total_decisions})"
        );
    }

    #[test]
    fn detects_multiple_ships_in_sequence() {
        // Two cargo vessels transit within the same recording.
        let mut sig = acoustic_signal::ambient_noise(200);
        sig.extend(acoustic_signal::ship_passage(400));
        sig.extend(acoustic_signal::ambient_noise(200));
        sig.extend(acoustic_signal::ship_passage(400));
        sig.extend(acoustic_signal::ambient_noise(100));

        let mut det = MarineDetector::new();
        let mut ships = 0;
        let mut ambients = 0;
        for &f in &sig {
            if let Some(ev) = det.step(f) {
                match ev {
                    AcousticEvent::ShipNoise => ships += 1,
                    AcousticEvent::Ambient => ambients += 1,
                    _ => {}
                }
            }
        }
        assert!(ships >= 6, "Expected >=6 ship windows across 2 passages, got {ships}");
        assert!(ambients >= 2, "Expected >=2 ambient windows between ships, got {ambients}");
    }

    #[test]
    fn fin_whale_call_during_ship_passage() {
        // A fin whale vocalising while a ship is passing by (real-ocean
        // scenario; the endangered species is often masked by traffic).
        let sig = acoustic_signal::fin_whale_under_ship(800);
        let mut det = MarineDetector::new();
        let mut fin = 0;
        let mut ship = 0;
        for &f in &sig {
            if let Some(ev) = det.step(f) {
                match ev {
                    AcousticEvent::FinWhale => fin += 1,
                    AcousticEvent::ShipNoise => ship += 1,
                    _ => {}
                }
            }
        }
        // Ship noise dominates, but the fin whale should surface at least
        // once during a 800-step mixed recording.
        assert!(ship >= 4, "Ship should dominate mixed scene: got {ship}");
        assert!(
            fin >= 1,
            "Detector should still flag a fin-whale call despite ship masking: got {fin}"
        );
    }

    #[test]
    fn sea_state_suppresses_false_ambient_alarms() {
        let mut det = MarineDetector::new();
        det.set_sea_state(6); // rough sea
        let sig = acoustic_signal::ambient_noise(500);
        let mut last = None;
        for &f in &sig {
            if let Some(e) = det.step(f) {
                last = Some(e);
            }
        }
        assert_eq!(
            last,
            Some(AcousticEvent::Ambient),
            "High sea state must not produce spurious whale detections on quiet data"
        );
    }

    #[test]
    fn classify_stream_csv() {
        let windows = acoustic_signal::from_csv("data/processed/sample_marine.csv");
        assert!(windows.len() >= 100);
        let mut det = MarineDetector::new();
        let results = det.classify_stream(&windows, 25);
        assert!(!results.is_empty(), "Should produce classifications");
        let has_ambient = results.iter().any(|r| r.event == AcousticEvent::Ambient);
        let has_ship = results.iter().any(|r| r.event == AcousticEvent::ShipNoise);
        assert!(has_ambient, "Should detect quiet ambient section");
        assert!(has_ship, "Should detect the ship-passage section");
    }

    #[test]
    fn confusion_matrix_accuracy() {
        let windows = acoustic_signal::from_csv("data/processed/sample_marine.csv");
        let mut det = MarineDetector::new();
        let preds = det.classify_stream(&windows, 25);
        let cm = ConfusionMatrix::from_predictions(&preds, &windows, 25);
        assert!(cm.total > 0);
        println!("UC03 CM accuracy: {:.1}%", cm.accuracy() * 100.0);
    }
}
