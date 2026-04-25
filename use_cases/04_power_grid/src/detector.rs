// SPDX-License-Identifier: AGPL-3.0-only
//! Power-grid event triage using a 4-channel CricketBrain `ResonatorBank`.
//!
//! Input: instantaneous dominant frequency (Hz) from a PMU short-time
//! spectrum.
//! Output: one of five `GridEvent` values per 50-step detection window.

use cricket_brain::resonator_bank::ResonatorBank;
use cricket_brain::token::TokenVocabulary;

/// Power-grid event categories triaged by this detector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridEvent {
    /// Total energy below the outage threshold — line is dead.
    Outage,
    /// 50 Hz fundamental dominant — healthy grid.
    Nominal,
    /// 100 Hz dominant — DC offset, transformer in-rush, half-wave loads.
    SecondHarmonic,
    /// 150 Hz dominant — non-linear loads (VFDs, SMPS, LED ballasts, arc).
    ThirdHarmonic,
    /// 200 Hz dominant — fast switching artefacts, EMI from RF power
    /// electronics.
    FourthHarmonic,
}

impl std::fmt::Display for GridEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridEvent::Outage => write!(f, "Outage"),
            GridEvent::Nominal => write!(f, "Nominal (50 Hz)"),
            GridEvent::SecondHarmonic => write!(f, "2nd Harmonic (100 Hz)"),
            GridEvent::ThirdHarmonic => write!(f, "3rd Harmonic (150 Hz)"),
            GridEvent::FourthHarmonic => write!(f, "4th Harmonic (200 Hz)"),
        }
    }
}

/// 4-channel power-grid event triage core.
pub struct GridDetector {
    bank: ResonatorBank,
    channel_energy: [f32; 4],
    window_size: usize,
    window_step: usize,
    last_event: GridEvent,
    last_confidence: f32,
    step_count: usize,
    /// Total-channel energy below which we declare `Outage`.
    outage_threshold: f32,
    /// v0.2 multi-label per-channel threshold for [`GridDetector::step_multi`].
    channel_threshold: f32,
}

const DEFAULT_OUTAGE_THRESHOLD: f32 = 0.1;
const DEFAULT_CHANNEL_THRESHOLD: f32 = 0.03;

impl GridDetector {
    /// Create a detector with channels at 50 / 100 / 150 / 200 Hz.
    pub fn new() -> Self {
        // TokenVocabulary distributes evenly in [50, 200], landing exactly
        // on each integer-multiple harmonic of the 50 Hz fundamental.
        let vocab = TokenVocabulary::new(
            &["FUND", "H2", "H3", "H4"],
            50.0,
            200.0,
        );
        let bank = ResonatorBank::new(&vocab);
        Self {
            bank,
            channel_energy: [0.0; 4],
            window_size: 50,
            window_step: 0,
            last_event: GridEvent::Outage,
            last_confidence: 0.0,
            step_count: 0,
            outage_threshold: DEFAULT_OUTAGE_THRESHOLD,
            channel_threshold: DEFAULT_CHANNEL_THRESHOLD,
        }
    }

    /// v0.2: create a detector with a wider Gaussian tuning so boundary
    /// frequencies (e.g. 75 Hz between Fund and H2) activate the nearest
    /// channel instead of falling through to `Outage`.
    ///
    /// Recommended bandwidth ≈ 0.20 (zero CSV regression in the marine
    /// equivalent). Library default is ~0.10.
    pub fn with_bandwidth(bandwidth: f32) -> Self {
        let mut det = Self::new();
        det.set_bandwidth(bandwidth);
        det
    }

    /// Set the Gaussian tuning bandwidth on every channel. Clamped to
    /// `[0.01, 0.80]`.
    pub fn set_bandwidth(&mut self, bandwidth: f32) {
        let bw = bandwidth.clamp(0.01, 0.80);
        for ch in &mut self.bank.channels {
            for n in &mut ch.neurons {
                n.bandwidth = bw;
            }
        }
    }

    /// v0.2 per-channel threshold used by [`GridDetector::step_multi`].
    pub fn set_channel_threshold(&mut self, threshold: f32) {
        self.channel_threshold = threshold.max(0.0);
    }

    /// Set the outage threshold explicitly (advanced use). The default
    /// (0.10) treats a window with negligible 50-200 Hz energy as
    /// `Outage`. Lower the threshold on a noisy grid where you want a
    /// stricter outage criterion.
    pub fn set_outage_threshold(&mut self, threshold: f32) {
        self.outage_threshold = threshold.max(0.0);
    }

    /// Feed one PMU dominant-frequency sample.
    pub fn step(&mut self, input_freq: f32) -> Option<GridEvent> {
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

    fn classify(&mut self) -> GridEvent {
        let total: f32 = self.channel_energy.iter().sum();
        if total < self.outage_threshold {
            self.last_confidence = 1.0;
            return GridEvent::Outage;
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

        // Token order: FUND=0, H2=1, H3=2, H4=3.
        match max_idx {
            0 => GridEvent::Nominal,
            1 => GridEvent::SecondHarmonic,
            2 => GridEvent::ThirdHarmonic,
            3 => GridEvent::FourthHarmonic,
            _ => GridEvent::Outage,
        }
    }

    pub fn confidence(&self) -> f32 {
        self.last_confidence
    }

    pub fn last_event(&self) -> GridEvent {
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
        self.last_event = GridEvent::Outage;
        self.last_confidence = 0.0;
        self.step_count = 0;
    }

    /// v0.2: feed one sample and, at the end of each 50-step window,
    /// return a [`MultiLabelDecision`] listing **every** channel whose
    /// accumulated energy exceeds [`GridDetector::channel_threshold`].
    ///
    /// Useful for the realistic case where a grid has a normal 50 Hz
    /// fundamental **and** simultaneous 3rd-harmonic distortion from
    /// connected non-linear loads — both labels surface together.
    pub fn step_multi(&mut self, input_freq: f32) -> Option<MultiLabelDecision> {
        let outputs = self.bank.step(input_freq);
        self.step_count += 1;
        self.window_step += 1;

        for (i, &out) in outputs.iter().enumerate().take(4) {
            if out > 0.0 {
                self.channel_energy[i] += out;
            }
        }

        if self.window_step >= self.window_size {
            let energies = self.channel_energy;
            let mut events = Vec::new();
            for (i, &e) in energies.iter().enumerate() {
                if e >= self.channel_threshold {
                    events.push(match i {
                        0 => GridEvent::Nominal,
                        1 => GridEvent::SecondHarmonic,
                        2 => GridEvent::ThirdHarmonic,
                        _ => GridEvent::FourthHarmonic,
                    });
                }
            }
            if events.is_empty() {
                events.push(GridEvent::Outage);
            }
            let decision = MultiLabelDecision {
                events,
                energies,
                step: self.step_count,
            };
            self.channel_energy = [0.0; 4];
            self.window_step = 0;
            return Some(decision);
        }

        None
    }
}

impl Default for GridDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// v0.2 multi-label detection result. `events` lists every channel above
/// the per-channel threshold; `energies` is a snapshot of the four
/// channels (FUND / H2 / H3 / H4).
#[derive(Debug, Clone)]
pub struct MultiLabelDecision {
    pub events: Vec<GridEvent>,
    pub energies: [f32; 4],
    pub step: usize,
}

// ---------------------------------------------------------------------------
// Batch classification & confusion matrix
// ---------------------------------------------------------------------------

use crate::grid_signal::GridWindow;

#[derive(Debug, Clone)]
pub struct WindowClassification {
    pub event: GridEvent,
    pub confidence: f32,
    pub step: usize,
}

impl GridDetector {
    /// Process a sequence of PMU windows. Each window's `dominant_freq`
    /// is fed for `steps_per_window` timesteps.
    pub fn classify_stream(
        &mut self,
        windows: &[GridWindow],
        steps_per_window: usize,
    ) -> Vec<WindowClassification> {
        self.reset();
        let stream = crate::grid_signal::windows_to_frequency_stream(windows, steps_per_window);
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

fn label_to_event(label: &str) -> GridEvent {
    match label.trim() {
        "Outage" => GridEvent::Outage,
        "Nominal" => GridEvent::Nominal,
        "SecondHarmonic" | "H2" => GridEvent::SecondHarmonic,
        "ThirdHarmonic" | "H3" => GridEvent::ThirdHarmonic,
        "FourthHarmonic" | "H4" => GridEvent::FourthHarmonic,
        _ => GridEvent::Outage,
    }
}

/// 5-class confusion matrix for grid events.
#[derive(Debug, Default)]
pub struct ConfusionMatrix {
    pub total: usize,
    pub correct: usize,
    pub tp_outage: usize,
    pub tp_nominal: usize,
    pub tp_h2: usize,
    pub tp_h3: usize,
    pub tp_h4: usize,
    pub fp_outage: usize,
    pub fp_nominal: usize,
    pub fp_h2: usize,
    pub fp_h3: usize,
    pub fp_h4: usize,
}

impl ConfusionMatrix {
    pub fn from_predictions(
        preds: &[WindowClassification],
        windows: &[GridWindow],
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
                (GridEvent::Outage, GridEvent::Outage) => cm.tp_outage += 1,
                (GridEvent::Outage, _) => cm.fp_outage += 1,
                (GridEvent::Nominal, GridEvent::Nominal) => cm.tp_nominal += 1,
                (GridEvent::Nominal, _) => cm.fp_nominal += 1,
                (GridEvent::SecondHarmonic, GridEvent::SecondHarmonic) => cm.tp_h2 += 1,
                (GridEvent::SecondHarmonic, _) => cm.fp_h2 += 1,
                (GridEvent::ThirdHarmonic, GridEvent::ThirdHarmonic) => cm.tp_h3 += 1,
                (GridEvent::ThirdHarmonic, _) => cm.fp_h3 += 1,
                (GridEvent::FourthHarmonic, GridEvent::FourthHarmonic) => cm.tp_h4 += 1,
                (GridEvent::FourthHarmonic, _) => cm.fp_h4 += 1,
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
        println!("  │              │ Outage │ Nominal│  H2    │  H3    │  H4    │");
        println!("  ├──────────────┼────────┼────────┼────────┼────────┼────────┤");
        println!(
            "  │ TP           │ {:>6} │ {:>6} │ {:>6} │ {:>6} │ {:>6} │",
            self.tp_outage, self.tp_nominal, self.tp_h2, self.tp_h3, self.tp_h4
        );
        println!(
            "  │ FP           │ {:>6} │ {:>6} │ {:>6} │ {:>6} │ {:>6} │",
            self.fp_outage, self.fp_nominal, self.fp_h2, self.fp_h3, self.fp_h4
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
    use crate::grid_signal;

    fn last_event(sig: &[f32]) -> Option<GridEvent> {
        let mut det = GridDetector::new();
        let mut last = None;
        for &f in sig {
            if let Some(e) = det.step(f) {
                last = Some(e);
            }
        }
        last
    }

    #[test]
    fn detects_outage() {
        let sig = grid_signal::outage(500);
        assert_eq!(last_event(&sig), Some(GridEvent::Outage));
    }

    #[test]
    fn detects_nominal() {
        let sig = grid_signal::nominal_grid(500);
        assert_eq!(last_event(&sig), Some(GridEvent::Nominal));
    }

    #[test]
    fn detects_second_harmonic() {
        let sig = grid_signal::second_harmonic_dominant(500);
        assert_eq!(last_event(&sig), Some(GridEvent::SecondHarmonic));
    }

    #[test]
    fn detects_third_harmonic() {
        let sig = grid_signal::third_harmonic_dominant(500);
        assert_eq!(last_event(&sig), Some(GridEvent::ThirdHarmonic));
    }

    #[test]
    fn detects_fourth_harmonic() {
        let sig = grid_signal::fourth_harmonic_dominant(500);
        assert_eq!(last_event(&sig), Some(GridEvent::FourthHarmonic));
    }

    #[test]
    fn detects_factory_startup_sequence() {
        // Pre-event: nominal. Mid-event: 3rd-harmonic distortion. Post: nominal.
        let sig = grid_signal::factory_startup(1500, 500);
        let mut det = GridDetector::new();
        let mut nominal_pre = 0;
        let mut h3_mid = 0;
        let mut nominal_post = 0;
        for &f in &sig {
            if let Some(ev) = det.step(f) {
                let step = det.steps_processed();
                if step <= 500 && ev == GridEvent::Nominal {
                    nominal_pre += 1;
                } else if step > 500 && step <= 1000 && ev == GridEvent::ThirdHarmonic {
                    h3_mid += 1;
                } else if step > 1000 && ev == GridEvent::Nominal {
                    nominal_post += 1;
                }
            }
        }
        assert!(nominal_pre >= 5, "Expected nominal pre-event windows, got {nominal_pre}");
        assert!(h3_mid >= 5, "Expected 3rd-harmonic mid-event windows, got {h3_mid}");
        assert!(nominal_post >= 5, "Expected nominal post-event windows, got {nominal_post}");
    }

    #[test]
    fn detects_rolling_brownout_dips() {
        let sig = grid_signal::rolling_brownout(2000, 4, 80);
        let mut det = GridDetector::new();
        let mut outages = 0;
        let mut nominals = 0;
        for &f in &sig {
            if let Some(ev) = det.step(f) {
                match ev {
                    GridEvent::Outage => outages += 1,
                    GridEvent::Nominal => nominals += 1,
                    _ => {}
                }
            }
        }
        assert!(outages >= 4, "Expected ≥4 outage windows from 4 brownout dips, got {outages}");
        assert!(nominals >= 20, "Expected nominal majority between dips, got {nominals}");
    }

    #[test]
    fn nominal_with_third_harmonic_picks_dominant() {
        // ~70 % fundamental, ~30 % 3rd-harmonic — Nominal should win the
        // single-label classification.
        let sig = grid_signal::nominal_with_third_harmonic(1000);
        let mut det = GridDetector::new();
        let mut nominal = 0;
        let mut h3 = 0;
        for &f in &sig {
            if let Some(ev) = det.step(f) {
                match ev {
                    GridEvent::Nominal => nominal += 1,
                    GridEvent::ThirdHarmonic => h3 += 1,
                    _ => {}
                }
            }
        }
        assert!(nominal > h3, "Nominal must dominate the 70/30 mix (nominal={nominal}, h3={h3})");
    }

    #[test]
    fn multi_label_recovers_both_in_mixed_grid() {
        // v0.2 multi-label path should report the 3rd harmonic alongside
        // the fundamental in the mixed-grid scene where the single-label
        // path only sees the dominant.
        let sig = grid_signal::nominal_with_third_harmonic(2000);
        let mut det = GridDetector::with_bandwidth(0.20);
        let mut both = 0;
        let mut nominal_only = 0;
        let mut h3_only = 0;
        let mut total = 0;
        for &f in &sig {
            if let Some(d) = det.step_multi(f) {
                total += 1;
                let has_n = d.events.contains(&GridEvent::Nominal);
                let has_h3 = d.events.contains(&GridEvent::ThirdHarmonic);
                match (has_n, has_h3) {
                    (true, true) => both += 1,
                    (true, false) => nominal_only += 1,
                    (false, true) => h3_only += 1,
                    _ => {}
                }
            }
        }
        assert!(total > 0);
        assert!(
            both >= 5,
            "v0.2 multi-label must report fund AND H3 together (got both={both}, nom_only={nominal_only}, h3_only={h3_only}, total={total})"
        );
    }

    #[test]
    fn classify_stream_csv() {
        let windows = grid_signal::from_csv("data/processed/sample_grid.csv");
        assert!(windows.len() >= 100);
        let mut det = GridDetector::new();
        let results = det.classify_stream(&windows, 25);
        assert!(!results.is_empty());
        let has_outage = results.iter().any(|r| r.event == GridEvent::Outage);
        let has_nominal = results.iter().any(|r| r.event == GridEvent::Nominal);
        let has_h3 = results.iter().any(|r| r.event == GridEvent::ThirdHarmonic);
        assert!(has_outage, "Should detect outage section");
        assert!(has_nominal, "Should detect nominal section");
        assert!(has_h3, "Should detect 3rd-harmonic section");
    }

    #[test]
    fn confusion_matrix_accuracy() {
        let windows = grid_signal::from_csv("data/processed/sample_grid.csv");
        let mut det = GridDetector::new();
        let preds = det.classify_stream(&windows, 25);
        let cm = ConfusionMatrix::from_predictions(&preds, &windows, 25);
        assert!(cm.total > 0);
        println!("UC04 CM accuracy: {:.1}%", cm.accuracy() * 100.0);
    }
}
