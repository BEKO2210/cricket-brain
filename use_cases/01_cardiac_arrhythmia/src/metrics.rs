// SPDX-License-Identifier: AGPL-3.0-only
//! Classification metrics for the cardiac rhythm-pattern triage benchmark.
//!
//! All metrics here operate on **explicit, externally provided ground-truth
//! labels** — they never derive ground truth from the detector's own
//! predictions. This is the core methodological fix for the previous
//! `ConfusionMatrix::from_predictions` which inferred truth from the
//! predicted BPM and was therefore partially circular.
//!
//! Supported classes (compile-time fixed):
//!     0 = NormalSinus
//!     1 = Tachycardia
//!     2 = Bradycardia
//!     3 = Irregular
//!
//! An additional optional **reject** outcome is supported for reject-aware
//! evaluation — see [`RejectAwareCounts`] / [`coverage_accuracy_curve`].
//!
//! All reported floats are in `f64` to keep aggregation stable across
//! large stress sweeps; per-step computations stay in `f32`.

use crate::detector::RhythmClass;

/// Number of supported ground-truth classes (Irregular is the 4th).
pub const NUM_CLASSES: usize = 4;

/// Stable index for a class. Used as the row/column index of the
/// confusion matrix.
#[inline]
pub fn class_index(c: RhythmClass) -> usize {
    match c {
        RhythmClass::NormalSinus => 0,
        RhythmClass::Tachycardia => 1,
        RhythmClass::Bradycardia => 2,
        RhythmClass::Irregular => 3,
    }
}

/// Inverse of [`class_index`] — used for table headers / reports only.
#[inline]
pub fn class_from_index(i: usize) -> RhythmClass {
    match i {
        0 => RhythmClass::NormalSinus,
        1 => RhythmClass::Tachycardia,
        2 => RhythmClass::Bradycardia,
        _ => RhythmClass::Irregular,
    }
}

/// Short stable label for a class — used in CSV / JSON output.
#[inline]
pub fn class_label(c: RhythmClass) -> &'static str {
    match c {
        RhythmClass::NormalSinus => "Normal",
        RhythmClass::Tachycardia => "Tachy",
        RhythmClass::Bradycardia => "Brady",
        RhythmClass::Irregular => "Irregular",
    }
}

/// Full 4×4 confusion matrix with rows = ground truth, cols = predicted.
#[derive(Debug, Clone, Default)]
pub struct ConfusionMatrix4 {
    /// `m[truth][pred]`
    pub m: [[u32; NUM_CLASSES]; NUM_CLASSES],
}

impl ConfusionMatrix4 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, truth: RhythmClass, pred: RhythmClass) {
        self.m[class_index(truth)][class_index(pred)] += 1;
    }

    pub fn total(&self) -> u32 {
        self.m.iter().flat_map(|row| row.iter()).copied().sum()
    }

    pub fn correct(&self) -> u32 {
        (0..NUM_CLASSES).map(|i| self.m[i][i]).sum()
    }

    /// Overall accuracy in [0, 1].
    pub fn accuracy(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        self.correct() as f64 / total as f64
    }

    /// Number of ground-truth samples in class `c`.
    pub fn support(&self, c: RhythmClass) -> u32 {
        self.m[class_index(c)].iter().copied().sum()
    }

    /// Number of predictions equal to class `c`.
    pub fn predicted(&self, c: RhythmClass) -> u32 {
        let i = class_index(c);
        (0..NUM_CLASSES).map(|t| self.m[t][i]).sum()
    }

    pub fn tp(&self, c: RhythmClass) -> u32 {
        let i = class_index(c);
        self.m[i][i]
    }

    pub fn fp(&self, c: RhythmClass) -> u32 {
        let i = class_index(c);
        let mut s = 0;
        for t in 0..NUM_CLASSES {
            if t != i {
                s += self.m[t][i];
            }
        }
        s
    }

    pub fn fn_(&self, c: RhythmClass) -> u32 {
        let i = class_index(c);
        let mut s = 0;
        for p in 0..NUM_CLASSES {
            if p != i {
                s += self.m[i][p];
            }
        }
        s
    }

    pub fn tn(&self, c: RhythmClass) -> u32 {
        let total = self.total();
        total
            .saturating_sub(self.tp(c))
            .saturating_sub(self.fp(c))
            .saturating_sub(self.fn_(c))
    }

    pub fn precision(&self, c: RhythmClass) -> f64 {
        let tp = self.tp(c) as f64;
        let fp = self.fp(c) as f64;
        if tp + fp == 0.0 {
            return 0.0;
        }
        tp / (tp + fp)
    }

    pub fn recall(&self, c: RhythmClass) -> f64 {
        let tp = self.tp(c) as f64;
        let fn_ = self.fn_(c) as f64;
        if tp + fn_ == 0.0 {
            return 0.0;
        }
        tp / (tp + fn_)
    }

    pub fn specificity(&self, c: RhythmClass) -> f64 {
        let tn = self.tn(c) as f64;
        let fp = self.fp(c) as f64;
        if tn + fp == 0.0 {
            return 0.0;
        }
        tn / (tn + fp)
    }

    pub fn f1(&self, c: RhythmClass) -> f64 {
        let p = self.precision(c);
        let r = self.recall(c);
        if p + r == 0.0 {
            return 0.0;
        }
        2.0 * p * r / (p + r)
    }

    /// Macro-F1 averaged over the classes that appear in the ground truth
    /// (classes with zero support are excluded so a missing class cannot
    /// silently push the macro average toward zero).
    pub fn macro_f1(&self) -> f64 {
        let mut sum = 0.0;
        let mut n = 0;
        for i in 0..NUM_CLASSES {
            let c = class_from_index(i);
            if self.support(c) == 0 {
                continue;
            }
            sum += self.f1(c);
            n += 1;
        }
        if n == 0 {
            0.0
        } else {
            sum / n as f64
        }
    }

    /// Weighted-F1 (weighted by ground-truth support).
    pub fn weighted_f1(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        let mut sum = 0.0;
        for i in 0..NUM_CLASSES {
            let c = class_from_index(i);
            let w = self.support(c) as f64 / total as f64;
            sum += w * self.f1(c);
        }
        sum
    }

    /// Balanced accuracy — mean of per-class recall over classes
    /// with non-zero support.
    pub fn balanced_accuracy(&self) -> f64 {
        let mut sum = 0.0;
        let mut n = 0;
        for i in 0..NUM_CLASSES {
            let c = class_from_index(i);
            if self.support(c) == 0 {
                continue;
            }
            sum += self.recall(c);
            n += 1;
        }
        if n == 0 {
            0.0
        } else {
            sum / n as f64
        }
    }

    /// Pretty-print as an ASCII table to stdout.
    pub fn print(&self, title: &str) {
        println!("\n{title}");
        println!("                 pred=Normal  pred=Tachy  pred=Brady  pred=Irreg");
        for t in 0..NUM_CLASSES {
            print!("  truth={:<9}", class_label(class_from_index(t)));
            for p in 0..NUM_CLASSES {
                print!(" {:>11}", self.m[t][p]);
            }
            println!();
        }
        println!(
            "  total={}  correct={}  accuracy={:.4}  macro_F1={:.4}  bal_acc={:.4}",
            self.total(),
            self.correct(),
            self.accuracy(),
            self.macro_f1(),
            self.balanced_accuracy(),
        );
    }

    /// Emit the matrix as CSV: `truth,Normal,Tachy,Brady,Irregular`.
    pub fn to_csv(&self) -> String {
        let mut out = String::from("truth,Normal,Tachy,Brady,Irregular\n");
        for t in 0..NUM_CLASSES {
            out.push_str(class_label(class_from_index(t)));
            for p in 0..NUM_CLASSES {
                out.push(',');
                out.push_str(&self.m[t][p].to_string());
            }
            out.push('\n');
        }
        out
    }
}

/// Per-class metric report row (for CSV / JSON output).
#[derive(Debug, Clone)]
pub struct PerClassMetrics {
    pub class: RhythmClass,
    pub support: u32,
    pub tp: u32,
    pub fp: u32,
    pub fn_: u32,
    pub tn: u32,
    pub precision: f64,
    pub recall: f64,
    pub specificity: f64,
    pub f1: f64,
}

impl PerClassMetrics {
    pub fn from_cm(cm: &ConfusionMatrix4, c: RhythmClass) -> Self {
        Self {
            class: c,
            support: cm.support(c),
            tp: cm.tp(c),
            fp: cm.fp(c),
            fn_: cm.fn_(c),
            tn: cm.tn(c),
            precision: cm.precision(c),
            recall: cm.recall(c),
            specificity: cm.specificity(c),
            f1: cm.f1(c),
        }
    }

    pub fn csv_header() -> &'static str {
        "class,support,tp,fp,fn,tn,precision,recall,specificity,f1"
    }

    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6}",
            class_label(self.class),
            self.support,
            self.tp,
            self.fp,
            self.fn_,
            self.tn,
            self.precision,
            self.recall,
            self.specificity,
            self.f1,
        )
    }
}

/// Aggregate metrics for the whole confusion matrix (machine-readable form).
#[derive(Debug, Clone)]
pub struct AggregateMetrics {
    pub total: u32,
    pub correct: u32,
    pub accuracy: f64,
    pub macro_f1: f64,
    pub weighted_f1: f64,
    pub balanced_accuracy: f64,
}

impl AggregateMetrics {
    pub fn from_cm(cm: &ConfusionMatrix4) -> Self {
        Self {
            total: cm.total(),
            correct: cm.correct(),
            accuracy: cm.accuracy(),
            macro_f1: cm.macro_f1(),
            weighted_f1: cm.weighted_f1(),
            balanced_accuracy: cm.balanced_accuracy(),
        }
    }
}

// ---------------------------------------------------------------------------
// Reject-aware metrics
// ---------------------------------------------------------------------------

/// Counts that distinguish covered / rejected predictions.
#[derive(Debug, Clone, Default)]
pub struct RejectAwareCounts {
    pub total: u32,
    pub covered: u32,
    pub rejected: u32,
    pub correct_covered: u32,
}

impl RejectAwareCounts {
    /// Coverage — fraction of samples for which the system did emit a
    /// (non-rejected) decision.
    pub fn coverage(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.covered as f64 / self.total as f64
    }

    /// Accuracy on the covered subset — i.e. the rate at which the
    /// system is correct **when it commits to a class**.
    pub fn covered_accuracy(&self) -> f64 {
        if self.covered == 0 {
            return 0.0;
        }
        self.correct_covered as f64 / self.covered as f64
    }

    /// Forced accuracy — what the system would score if every reject
    /// were treated as a wrong answer (worst-case).
    pub fn forced_accuracy(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.correct_covered as f64 / self.total as f64
    }
}

/// One point on a coverage / accuracy curve.
#[derive(Debug, Clone)]
pub struct CoveragePoint {
    pub confidence_threshold: f32,
    pub coverage: f64,
    pub covered_accuracy: f64,
    pub forced_accuracy: f64,
    pub covered: u32,
    pub correct_covered: u32,
}

/// Build a coverage / accuracy curve: for a sweep of confidence
/// thresholds, count how often the detector would have committed and
/// how often it was right when committing.
///
/// `samples` is `(truth, pred, confidence)`; the system rejects when
/// `confidence < threshold`.
pub fn coverage_accuracy_curve(
    samples: &[(RhythmClass, RhythmClass, f32)],
    thresholds: &[f32],
) -> Vec<CoveragePoint> {
    let mut out = Vec::with_capacity(thresholds.len());
    for &thr in thresholds {
        let mut counts = RejectAwareCounts::default();
        counts.total = samples.len() as u32;
        for &(truth, pred, conf) in samples {
            if conf < thr {
                counts.rejected += 1;
            } else {
                counts.covered += 1;
                if truth == pred {
                    counts.correct_covered += 1;
                }
            }
        }
        out.push(CoveragePoint {
            confidence_threshold: thr,
            coverage: counts.coverage(),
            covered_accuracy: counts.covered_accuracy(),
            forced_accuracy: counts.forced_accuracy(),
            covered: counts.covered,
            correct_covered: counts.correct_covered,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cm_balanced() -> ConfusionMatrix4 {
        let mut cm = ConfusionMatrix4::new();
        // 100 perfect Normal predictions
        for _ in 0..100 {
            cm.record(RhythmClass::NormalSinus, RhythmClass::NormalSinus);
        }
        // 100 perfect Tachy predictions
        for _ in 0..100 {
            cm.record(RhythmClass::Tachycardia, RhythmClass::Tachycardia);
        }
        cm
    }

    #[test]
    fn perfect_balanced_matrix() {
        let cm = cm_balanced();
        assert_eq!(cm.total(), 200);
        assert_eq!(cm.correct(), 200);
        assert!((cm.accuracy() - 1.0).abs() < 1e-9);
        assert!((cm.macro_f1() - 1.0).abs() < 1e-9);
        assert!((cm.balanced_accuracy() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn precision_recall_basic() {
        let mut cm = ConfusionMatrix4::new();
        // truth=Normal: 8 correct, 2 mislabelled as Tachy
        for _ in 0..8 {
            cm.record(RhythmClass::NormalSinus, RhythmClass::NormalSinus);
        }
        for _ in 0..2 {
            cm.record(RhythmClass::NormalSinus, RhythmClass::Tachycardia);
        }
        // truth=Tachy: 1 mislabelled as Normal
        cm.record(RhythmClass::Tachycardia, RhythmClass::NormalSinus);

        // Normal: TP=8, FP=1, FN=2  → P=8/9, R=8/10
        let p = cm.precision(RhythmClass::NormalSinus);
        let r = cm.recall(RhythmClass::NormalSinus);
        assert!((p - 8.0 / 9.0).abs() < 1e-6);
        assert!((r - 8.0 / 10.0).abs() < 1e-6);
        // Macro F1 averages across classes-with-support; Brady & Irreg
        // have zero support so are excluded.
        let m = cm.macro_f1();
        assert!(m > 0.0 && m < 1.0);
    }

    #[test]
    fn reject_curve_monotone_at_extremes() {
        // synth: 100 samples, 80% correct, confidence in [0,1]
        let mut samples = Vec::new();
        for i in 0..100 {
            let pred = if i < 80 {
                RhythmClass::NormalSinus
            } else {
                RhythmClass::Tachycardia
            };
            // higher confidence on the correct ones
            let conf = if i < 80 { 0.9 } else { 0.4 };
            samples.push((RhythmClass::NormalSinus, pred, conf));
        }
        let curve = coverage_accuracy_curve(&samples, &[0.0, 0.5, 1.0]);
        // thr=0.0 covers everything → covered_accuracy = 0.8
        assert!((curve[0].coverage - 1.0).abs() < 1e-9);
        assert!((curve[0].covered_accuracy - 0.8).abs() < 1e-9);
        // thr=0.5 keeps only the high-confidence (correct) 80
        assert!((curve[1].coverage - 0.8).abs() < 1e-9);
        assert!((curve[1].covered_accuracy - 1.0).abs() < 1e-9);
        // thr=1.0 rejects all
        assert!((curve[2].coverage - 0.0).abs() < 1e-9);
        assert!((curve[2].covered_accuracy - 0.0).abs() < 1e-9);
    }

    #[test]
    fn csv_header_stable() {
        assert!(PerClassMetrics::csv_header().starts_with("class,"));
    }
}
