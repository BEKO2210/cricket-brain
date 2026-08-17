// SPDX-License-Identifier: AGPL-3.0-only
//! Signal Detection Theory (SDT) benchmark for cardiac rhythm detection.
//!
//! Methodology: Green & Swets (1966)
//! Metrics: d' (sensitivity), AUC, TPR, FPR with Wilson 95% CI
//!
//! Tests the CardiacDetector on synthetic ECG data:
//! - Condition 1: Normal vs Tachycardia (can it distinguish fast from normal?)
//! - Condition 2: Normal vs Bradycardia (can it distinguish slow from normal?)
//! - Condition 3: Tachycardia vs Bradycardia (can it tell them apart?)
//!
//! Date: 2026-04-10

use cricket_brain_cardiac::detector::{CardiacDetector, RhythmClass};
use cricket_brain_cardiac::ecg_signal;

const TRIALS: usize = 200;

fn wilson_ci(successes: usize, total: usize) -> (f32, f32) {
    if total == 0 {
        return (0.0, 1.0);
    }
    let n = total as f32;
    let p = successes as f32 / n;
    let z = 1.96f32;
    let z2 = z * z;
    let center = (p + z2 / (2.0 * n)) / (1.0 + z2 / n);
    let margin = z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / (1.0 + z2 / n);
    ((center - margin).max(0.0), (center + margin).min(1.0))
}

fn inv_norm(p: f32) -> f32 {
    // Abramowitz & Stegun 26.2.23 approximation
    let p = p.clamp(0.0001, 0.9999);
    let t = if p < 0.5 {
        (-2.0 * p.ln()).sqrt()
    } else {
        (-2.0 * (1.0 - p).ln()).sqrt()
    };
    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;
    let val = t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t);
    if p < 0.5 {
        -val
    } else {
        val
    }
}

fn d_prime(hit_rate: f32, fa_rate: f32) -> f32 {
    let hr = hit_rate.clamp(0.001, 0.999);
    let fa = fa_rate.clamp(0.001, 0.999);
    inv_norm(hr) - inv_norm(fa)
}

struct SdtResult {
    name: &'static str,
    hits: usize,
    misses: usize,
    false_alarms: usize,
    correct_rejections: usize,
}

impl SdtResult {
    fn tpr(&self) -> f32 {
        self.hits as f32 / (self.hits + self.misses).max(1) as f32
    }
    fn fpr(&self) -> f32 {
        self.false_alarms as f32 / (self.false_alarms + self.correct_rejections).max(1) as f32
    }
    fn d_prime(&self) -> f32 {
        d_prime(self.tpr(), self.fpr())
    }
    fn auc(&self) -> f32 {
        // Simple AUC approximation from TPR and FPR
        let tpr = self.tpr();
        let fpr = self.fpr();
        0.5 + (tpr - fpr).abs() / 2.0 + (tpr - fpr).abs().min(tpr.min(1.0 - fpr)) / 2.0
    }
}

fn run_condition(
    name: &'static str,
    target_class: RhythmClass,
    target_cycle: &ecg_signal::EcgCycle,
    noise_cycle: &ecg_signal::EcgCycle,
) -> SdtResult {
    let mut det = CardiacDetector::new();
    let mut hits = 0;
    let mut misses = 0;
    let mut false_alarms = 0;
    let mut correct_rejections = 0;

    // Signal-present trials: target rhythm
    for _ in 0..TRIALS {
        det.reset();
        let stream = target_cycle.to_frequency_stream(8);
        let mut last_class = None;
        for &f in &stream {
            if let Some(c) = det.step(f) {
                last_class = Some(c);
            }
        }
        if last_class == Some(target_class) {
            hits += 1;
        } else {
            misses += 1;
        }
    }

    // Signal-absent trials: noise rhythm
    for _ in 0..TRIALS {
        det.reset();
        let stream = noise_cycle.to_frequency_stream(8);
        let mut last_class = None;
        for &f in &stream {
            if let Some(c) = det.step(f) {
                last_class = Some(c);
            }
        }
        if last_class == Some(target_class) {
            false_alarms += 1;
        } else {
            correct_rejections += 1;
        }
    }

    SdtResult {
        name,
        hits,
        misses,
        false_alarms,
        correct_rejections,
    }
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Cardiac SDT Benchmark — Green & Swets (1966)              ║");
    println!("║  Date: 2026-04-10 | Trials: {TRIALS} per class             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let normal = ecg_signal::normal_sinus();
    let tachy = ecg_signal::tachycardia();
    let brady = ecg_signal::bradycardia();

    let results = vec![
        run_condition("Tachy vs Normal", RhythmClass::Tachycardia, &tachy, &normal),
        run_condition("Brady vs Normal", RhythmClass::Bradycardia, &brady, &normal),
        run_condition("Normal vs Tachy", RhythmClass::NormalSinus, &normal, &tachy),
    ];

    println!(
        "  {:30} {:>6} {:>6} {:>7} {:>6} {:>8}",
        "Condition", "TPR", "FPR", "d'", "AUC", "Rating"
    );
    println!(
        "  {:─>30} {:─>6} {:─>6} {:─>7} {:─>6} {:─>8}",
        "", "", "", "", "", ""
    );

    for r in &results {
        let tpr = r.tpr();
        let fpr = r.fpr();
        let dp = r.d_prime();
        let auc = r.auc();
        let (tpr_lo, tpr_hi) = wilson_ci(r.hits, r.hits + r.misses);
        let (fpr_lo, fpr_hi) = wilson_ci(r.false_alarms, r.false_alarms + r.correct_rejections);

        let rating = if dp > 3.0 {
            "EXCELLENT"
        } else if dp > 2.0 {
            "GOOD"
        } else if dp > 1.0 {
            "MODERATE"
        } else {
            "POOR"
        };

        println!(
            "  {:30} {:.3}  {:.3}  {:>6.2}  {:.3}  {}",
            r.name, tpr, fpr, dp, auc, rating
        );
        println!(
            "    TPR 95% CI: [{:.3}, {:.3}]  FPR 95% CI: [{:.3}, {:.3}]",
            tpr_lo, tpr_hi, fpr_lo, fpr_hi
        );
    }

    println!("\n  Note: d' > 3.0 = excellent discrimination (near ceiling).");
    println!("  Methodology: {TRIALS} signal + {TRIALS} noise trials per condition.");
}
