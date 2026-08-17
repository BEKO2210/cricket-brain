// SPDX-License-Identifier: AGPL-3.0-only
//! Glue between the synthetic-recording ground truth, the detector
//! predictions, and the metrics module.
//!
//! This file deliberately has **no scoring of its own** — it only
//! pairs `(truth, prediction, confidence)` triples and hands them to
//! [`crate::metrics`].

use crate::detector::{BeatClassification, CardiacDetector, RhythmClass};
use crate::metrics::ConfusionMatrix4;
use crate::synthetic::SyntheticRecording;

/// One scored detector emission against the labelled stream.
#[derive(Debug, Clone)]
pub struct ScoredEmission {
    pub step: usize,
    pub truth: RhythmClass,
    pub pred: RhythmClass,
    pub confidence: f32,
    pub bpm: f32,
}

/// Run the detector over an entire synthetic recording and pair every
/// emitted classification with the ground-truth label of the segment
/// in which the emission occurred.
///
/// Emissions that fall outside any segment (only possible for empty
/// recordings) are silently dropped.
pub fn run_and_score(
    detector: &mut CardiacDetector,
    rec: &SyntheticRecording,
) -> Vec<ScoredEmission> {
    detector.reset();
    let mut out = Vec::new();
    for (i, &freq) in rec.stream.iter().enumerate() {
        if let Some(class) = detector.step(freq) {
            if let Some(truth) = rec.label_for_step(i) {
                out.push(ScoredEmission {
                    step: i,
                    truth,
                    pred: class,
                    confidence: detector.confidence(),
                    bpm: detector.bpm_estimate(),
                });
            }
        }
    }
    out
}

/// Convenience: build a confusion matrix straight from the scored
/// emissions.
pub fn confusion_matrix(scored: &[ScoredEmission]) -> ConfusionMatrix4 {
    let mut cm = ConfusionMatrix4::new();
    for s in scored {
        cm.record(s.truth, s.pred);
    }
    cm
}

/// Convenience: lift scored emissions into the
/// `(truth, pred, confidence)` shape used by
/// [`crate::metrics::coverage_accuracy_curve`].
pub fn to_truth_pred_conf(scored: &[ScoredEmission]) -> Vec<(RhythmClass, RhythmClass, f32)> {
    scored
        .iter()
        .map(|s| (s.truth, s.pred, s.confidence))
        .collect()
}

/// Drop the first `warmup_emissions` predictions from each segment.
/// Detector RR-window classifiers always need a few beats to converge.
/// Reporting metrics that include this warmup is honest only if the
/// warmup is also reported separately.
pub fn drop_warmup(scored: &[ScoredEmission], warmup_emissions: usize) -> Vec<ScoredEmission> {
    if warmup_emissions == 0 {
        return scored.to_vec();
    }
    let mut out = Vec::with_capacity(scored.len());
    let mut current_truth: Option<RhythmClass> = None;
    let mut seen_in_segment: usize = 0;
    for s in scored {
        if Some(s.truth) != current_truth {
            current_truth = Some(s.truth);
            seen_in_segment = 0;
        }
        seen_in_segment += 1;
        if seen_in_segment > warmup_emissions {
            out.push(s.clone());
        }
    }
    out
}

/// Construct a `BeatClassification`-shaped trace from scored emissions
/// (handy for reusing existing reporting code).
pub fn to_beat_classifications(scored: &[ScoredEmission]) -> Vec<BeatClassification> {
    scored
        .iter()
        .map(|s| BeatClassification {
            rhythm: s.pred,
            confidence: s.confidence,
            bpm: s.bpm,
            step: s.step,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::{generate, SyntheticConfig};

    #[test]
    fn scoring_lines_up_with_segments() {
        let cfg = SyntheticConfig::default()
            .with_seed(3)
            .with_beats_per_class(8)
            .with_irregular(false);
        let rec = generate(&cfg);
        let mut det = CardiacDetector::new();
        let scored = run_and_score(&mut det, &rec);
        for s in &scored {
            // Ground-truth must be the segment that contains the emission step.
            let seg = rec
                .segments
                .iter()
                .find(|seg| seg.contains(s.step))
                .unwrap();
            assert_eq!(s.truth, seg.class);
        }
    }
}
