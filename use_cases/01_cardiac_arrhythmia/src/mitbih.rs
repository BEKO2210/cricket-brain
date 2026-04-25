// SPDX-License-Identifier: AGPL-3.0-only
//! MIT-BIH ingest helpers — AAMI beat-type mapping, rate-regime ground
//! truth from RR windows, and patient-level evaluation primitives.
//!
//! ## Important honesty notice
//!
//! `cricket_brain_cardiac` is a **rate-regime triage** core. The
//! [`crate::detector::CardiacDetector`] classifies a sliding RR-interval
//! window into Normal / Tachy / Brady / Irregular. It does **not**
//! recognise AAMI beat *morphology* classes (N / S / V / F / Q). This
//! module therefore:
//!
//! 1. Provides an [`AamiClass`] mapping from MIT-BIH symbols to the AAMI
//!    5-class grouping (Moody & Mark 2001, AAMI EC57:2012, Table 1).
//!    The mapping is exposed for *traceability and per-symbol failure
//!    analysis only* — it is **not** the rate-regime ground truth.
//! 2. Provides [`rate_regime_truth`] — a sliding-window function that
//!    derives the rate-regime ground truth from real annotation
//!    timestamps. The window deliberately uses parameters that differ
//!    from the detector's internal RR window so the ground truth is
//!    not trivially circular with the detector output.
//! 3. Provides [`PerRecordResult`] / [`PooledResult`] for honest
//!    patient-level reporting.
//!
//! No real MIT-BIH data is shipped with the repo. The
//! `cardiac_mitbih` benchmark refuses to publish "validated" numbers
//! when only the synthetic sample is present.

use crate::detector::RhythmClass;

/// AAMI EC57:2012 inter-patient training split (DS1) — 22 MIT-BIH
/// records reserved for training morphology classifiers (de Chazal et
/// al. 2004, AAMI EC57:2012). CricketBrain is zero-training so DS1 is
/// not required for evaluation, but is exposed here for symmetry and
/// future ablation work.
pub const AAMI_DS1: &[&str] = &[
    "101", "106", "108", "109", "112", "114", "115", "116", "118", "119", "122", "124", "201",
    "203", "205", "207", "208", "209", "215", "220", "223", "230",
];

/// AAMI EC57:2012 inter-patient testing split (DS2) — 22 MIT-BIH
/// records that constitute the canonical inter-patient test set in
/// the published ECG literature.
pub const AAMI_DS2: &[&str] = &[
    "100", "103", "105", "111", "113", "117", "121", "123", "200", "202", "210", "212", "213",
    "214", "219", "221", "222", "228", "231", "232", "233", "234",
];

/// Records excluded from AAMI EC57:2012 evaluation because they
/// contain extensively paced beats (the AAMI standard removes them).
pub const AAMI_EXCLUDED_PACED: &[&str] = &["102", "104", "107", "217"];

/// Convenience: which AAMI bucket a record belongs to.
pub fn aami_split_for(record_id: &str) -> &'static str {
    if AAMI_DS1.contains(&record_id) {
        "DS1"
    } else if AAMI_DS2.contains(&record_id) {
        "DS2"
    } else if AAMI_EXCLUDED_PACED.contains(&record_id) {
        "excluded_paced"
    } else {
        "other"
    }
}

/// AAMI 5-class beat groupings (per AAMI EC57:2012, Moody & Mark 2001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AamiClass {
    /// Any normal / supraventricular non-ectopic beat.
    Normal,
    /// Supraventricular ectopic beat.
    Supraventricular,
    /// Ventricular ectopic beat.
    Ventricular,
    /// Fusion of normal and ventricular beat.
    Fusion,
    /// Unknown / paced.
    Unknown,
}

impl AamiClass {
    /// Stable short label, used in CSV / JSON columns.
    pub fn label(self) -> &'static str {
        match self {
            AamiClass::Normal => "N",
            AamiClass::Supraventricular => "S",
            AamiClass::Ventricular => "V",
            AamiClass::Fusion => "F",
            AamiClass::Unknown => "Q",
        }
    }
}

/// Map a MIT-BIH PhysioBank annotation symbol to the AAMI 5-class
/// grouping. Returns `None` for non-beat annotations (rhythm change
/// markers, signal-quality flags, etc.).
///
/// Source: AAMI EC57:2012 Table 1, Moody & Mark (2001), wfdb manual.
pub fn aami_from_symbol(sym: &str) -> Option<AamiClass> {
    use AamiClass::*;
    let c = match sym {
        // --- Normal (N): N, L, R, e, j ---
        "N" | "L" | "R" | "e" | "j" => Normal,
        // --- Supraventricular ectopic (S): A, a, J, S ---
        "A" | "a" | "J" | "S" => Supraventricular,
        // --- Ventricular ectopic (V): V, E ---
        "V" | "E" => Ventricular,
        // --- Fusion (F): F ---
        "F" => Fusion,
        // --- Unknown / paced (Q): /, f, Q ---
        "/" | "f" | "Q" => Unknown,
        _ => return None,
    };
    Some(c)
}

/// Configuration for the rate-regime ground-truth window used on real
/// MIT-BIH annotation streams.
///
/// We deliberately differ from the detector's internal 8-beat window
/// (which is configured in [`crate::detector::CardiacDetector`]) so
/// that "ground truth" and "prediction" can disagree on transition
/// zones — that disagreement is what the benchmark needs to measure.
#[derive(Debug, Clone, Copy)]
pub struct RateRegimeWindow {
    /// Number of preceding RR intervals to average over.
    pub window_beats: usize,
    /// CV(RR) above which the window is labelled `Irregular`.
    pub cv_irregular: f32,
    /// BPM strict-greater-than threshold for `Tachycardia`.
    pub bpm_tachy: f32,
    /// BPM strict-less-than threshold for `Bradycardia`.
    pub bpm_brady: f32,
}

impl Default for RateRegimeWindow {
    fn default() -> Self {
        // Window = 5 beats, intentionally smaller than the detector's 8.
        // Thresholds match the detector so a correctly-converged detector
        // should score well — but the smaller window means transitions
        // are visible.
        Self {
            window_beats: 5,
            cv_irregular: 0.30,
            bpm_tachy: 100.0,
            bpm_brady: 60.0,
        }
    }
}

/// Derive a rate-regime ground-truth label for the *i*-th beat in
/// `rr_intervals_ms`, given the window config.
///
/// Returns `None` when there are fewer than `window_beats` RR
/// intervals available (warmup at the start of a record).
pub fn rate_regime_truth(
    rr_intervals_ms: &[u32],
    i: usize,
    win: &RateRegimeWindow,
) -> Option<RhythmClass> {
    if win.window_beats < 2 {
        return None;
    }
    if i + 1 < win.window_beats {
        return None;
    }
    let start = i + 1 - win.window_beats;
    let slice = &rr_intervals_ms[start..=i];
    let n = slice.len() as f32;
    let sum: u64 = slice.iter().map(|&v| v as u64).sum();
    let mean = sum as f32 / n;
    if mean <= 0.0 {
        return None;
    }
    let bpm = 60_000.0 / mean;
    let var = slice
        .iter()
        .map(|&v| {
            let d = v as f32 - mean;
            d * d
        })
        .sum::<f32>()
        / n;
    let cv = var.sqrt() / mean;

    let class = if cv > win.cv_irregular {
        RhythmClass::Irregular
    } else if bpm > win.bpm_tachy {
        RhythmClass::Tachycardia
    } else if bpm < win.bpm_brady {
        RhythmClass::Bradycardia
    } else {
        RhythmClass::NormalSinus
    };
    Some(class)
}

/// One record's evaluation result — kept separate from any other
/// record's result to preserve patient-level visibility. Pooling
/// across records happens in a separate aggregation step.
#[derive(Debug, Clone)]
pub struct PerRecordResult {
    pub record_id: String,
    pub n_beats: usize,
    pub n_emissions: usize,
    pub n_ground_truth: usize,
    pub n_correct: usize,
    /// Distribution of AAMI symbols in this record.
    pub aami_counts: [u32; 5],
}

impl PerRecordResult {
    pub fn accuracy(&self) -> f64 {
        if self.n_ground_truth == 0 {
            0.0
        } else {
            self.n_correct as f64 / self.n_ground_truth as f64
        }
    }
}

/// Aggregate per-record results without losing patient-level structure.
///
/// We report **macro-average over records** (each record counts equally,
/// regardless of beat count) so a single long record cannot dominate
/// the headline number.
#[derive(Debug, Clone, Default)]
pub struct PooledResult {
    pub records: Vec<PerRecordResult>,
}

impl PooledResult {
    pub fn macro_accuracy(&self) -> f64 {
        if self.records.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.records.iter().map(|r| r.accuracy()).sum();
        sum / self.records.len() as f64
    }

    pub fn micro_accuracy(&self) -> f64 {
        let total_truth: usize = self.records.iter().map(|r| r.n_ground_truth).sum();
        if total_truth == 0 {
            return 0.0;
        }
        let total_correct: usize = self.records.iter().map(|r| r.n_correct).sum();
        total_correct as f64 / total_truth as f64
    }

    pub fn total_beats(&self) -> usize {
        self.records.iter().map(|r| r.n_beats).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aami_basic_symbols() {
        assert_eq!(aami_from_symbol("N"), Some(AamiClass::Normal));
        assert_eq!(aami_from_symbol("L"), Some(AamiClass::Normal));
        assert_eq!(aami_from_symbol("R"), Some(AamiClass::Normal));
        assert_eq!(aami_from_symbol("A"), Some(AamiClass::Supraventricular));
        assert_eq!(aami_from_symbol("V"), Some(AamiClass::Ventricular));
        assert_eq!(aami_from_symbol("F"), Some(AamiClass::Fusion));
        assert_eq!(aami_from_symbol("/"), Some(AamiClass::Unknown));
    }

    #[test]
    fn aami_non_beat_symbols_are_none() {
        // `+` = rhythm change, `~` = signal quality, `|` = isolated
        // QRS-like artifact: not beats.
        assert_eq!(aami_from_symbol("+"), None);
        assert_eq!(aami_from_symbol("~"), None);
        assert_eq!(aami_from_symbol("|"), None);
        assert_eq!(aami_from_symbol(""), None);
        assert_eq!(aami_from_symbol("?"), None);
    }

    #[test]
    fn aami_label_round_trip() {
        for &s in &["N", "L", "R", "A", "V", "F", "/", "f"] {
            let c = aami_from_symbol(s).unwrap();
            assert!(["N", "S", "V", "F", "Q"].contains(&c.label()));
        }
    }

    #[test]
    fn rate_regime_normal() {
        // Five RRs around 800 ms → ~75 BPM, low CV → Normal.
        let rr = [800u32, 810, 790, 800, 805];
        let win = RateRegimeWindow::default();
        let cls = rate_regime_truth(&rr, 4, &win).unwrap();
        assert_eq!(cls, RhythmClass::NormalSinus);
    }

    #[test]
    fn rate_regime_tachy() {
        let rr = [400u32, 410, 390, 400, 405];
        let win = RateRegimeWindow::default();
        let cls = rate_regime_truth(&rr, 4, &win).unwrap();
        assert_eq!(cls, RhythmClass::Tachycardia);
    }

    #[test]
    fn rate_regime_brady() {
        let rr = [1500u32, 1510, 1490, 1500, 1505];
        let win = RateRegimeWindow::default();
        let cls = rate_regime_truth(&rr, 4, &win).unwrap();
        assert_eq!(cls, RhythmClass::Bradycardia);
    }

    #[test]
    fn rate_regime_irregular_high_cv() {
        // Wildly varying RRs → high CV → Irregular.
        let rr = [400u32, 1200, 350, 1500, 380];
        let win = RateRegimeWindow::default();
        let cls = rate_regime_truth(&rr, 4, &win).unwrap();
        assert_eq!(cls, RhythmClass::Irregular);
    }

    #[test]
    fn rate_regime_warmup_returns_none() {
        let rr = [800u32, 810];
        let win = RateRegimeWindow::default();
        assert!(rate_regime_truth(&rr, 0, &win).is_none());
        assert!(rate_regime_truth(&rr, 1, &win).is_none());
    }

    #[test]
    fn aami_splits_disjoint_and_complete() {
        // DS1 ∩ DS2 = ∅
        for r in AAMI_DS1 {
            assert!(!AAMI_DS2.contains(r), "{} appears in both DS1 and DS2", r);
            assert!(
                !AAMI_EXCLUDED_PACED.contains(r),
                "{} is in DS1 but also excluded",
                r
            );
        }
        for r in AAMI_DS2 {
            assert!(!AAMI_EXCLUDED_PACED.contains(r));
        }
        // 22 + 22 + 4 = 48 (full MIT-BIH record count)
        assert_eq!(
            AAMI_DS1.len() + AAMI_DS2.len() + AAMI_EXCLUDED_PACED.len(),
            48
        );
    }

    #[test]
    fn aami_split_for_known_records() {
        assert_eq!(aami_split_for("101"), "DS1");
        assert_eq!(aami_split_for("100"), "DS2");
        assert_eq!(aami_split_for("217"), "excluded_paced");
        assert_eq!(aami_split_for("999"), "other");
    }

    #[test]
    fn pooled_macro_vs_micro() {
        // Record A: 1/1 correct → 100 %. Record B: 1/9 correct → ~11 %.
        // Macro = 55.5 %, micro = 2/10 = 20 %.
        let r_a = PerRecordResult {
            record_id: "A".into(),
            n_beats: 1,
            n_emissions: 1,
            n_ground_truth: 1,
            n_correct: 1,
            aami_counts: [1, 0, 0, 0, 0],
        };
        let r_b = PerRecordResult {
            record_id: "B".into(),
            n_beats: 9,
            n_emissions: 9,
            n_ground_truth: 9,
            n_correct: 1,
            aami_counts: [9, 0, 0, 0, 0],
        };
        let pooled = PooledResult {
            records: vec![r_a, r_b],
        };
        assert!((pooled.macro_accuracy() - 0.5556).abs() < 0.001);
        assert!((pooled.micro_accuracy() - 0.2).abs() < 0.001);
    }
}
