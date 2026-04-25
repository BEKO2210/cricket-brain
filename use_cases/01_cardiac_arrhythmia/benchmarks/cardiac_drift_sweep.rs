// SPDX-License-Identifier: AGPL-3.0-only
//! Carrier-frequency drift robustness sweep (v0.6).
//!
//! ## Hypothesis under test
//!
//! On AAMI DS2 / synthetic clean data, the simple `ThresholdBurstBaseline`
//! is on par with CricketBrain (~0.5 pp better on a clean v0.5 run).
//! The rule has a **hard ±10 % band-pass** centred at 4500 Hz; once the
//! QRS carrier moves outside that band, the rule simply stops emitting.
//! CricketBrain has **Gaussian tuning** plus an adaptive sensitivity (AGC)
//! layer, so its response should degrade smoothly rather than fall off
//! a cliff.
//!
//! This benchmark sweeps the QRS carrier frequency offset from -20 % to
//! +20 % of the nominal 4500 Hz on a deterministic synthetic recording
//! and records, for each system:
//!
//! * total emissions
//! * accuracy on the labelled segments
//! * Brady / Normal / Tachy / Irregular per-class recall
//!
//! Reproduce:
//! ```
//! cargo run --release --example cardiac_drift_sweep -- --seed 42 --write
//! ```
//!
//! Output (when `--write`):
//! `results/cardiac_drift_sweep.csv` with one row per
//! (carrier_offset_pct, system).

use std::path::PathBuf;

use cricket_brain_cardiac::baselines::{
    BaselinePrediction, FrequencyRuleBaseline, ThresholdBurstBaseline,
};
use cricket_brain_cardiac::detector::{CardiacDetector, RhythmClass};
use cricket_brain_cardiac::evaluate::{drop_warmup, ScoredEmission};
use cricket_brain_cardiac::metrics::{
    class_from_index, class_label, AggregateMetrics, ConfusionMatrix4, NUM_CLASSES,
};
use cricket_brain_cardiac::report::{current_command_line, write_csv_with_header, RunMetadata};
use cricket_brain_cardiac::synthetic::{generate, SyntheticConfig, SyntheticRecording};

const RESULT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/results");
const NOMINAL_QRS_HZ: f32 = 4500.0;

struct Args {
    seed: u64,
    beats_per_class: u32,
    warmup: usize,
    write: bool,
}

impl Args {
    fn parse() -> Self {
        let mut a = Args {
            seed: 42,
            beats_per_class: 25,
            warmup: 2,
            write: false,
        };
        let mut iter = std::env::args().skip(1);
        while let Some(flag) = iter.next() {
            match flag.as_str() {
                "--seed" => a.seed = iter.next().unwrap_or_default().parse().unwrap_or(a.seed),
                "--beats-per-class" => {
                    a.beats_per_class = iter
                        .next()
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or(a.beats_per_class)
                }
                "--warmup" => {
                    a.warmup = iter.next().unwrap_or_default().parse().unwrap_or(a.warmup)
                }
                "--write" => a.write = true,
                _ => {
                    eprintln!("Unknown flag: {flag}");
                    std::process::exit(2);
                }
            }
        }
        a
    }
}

/// Apply a carrier-frequency offset to a frequency stream: every
/// non-zero sample is scaled by `(1 + offset)`. Silence is preserved.
fn apply_carrier_drift(stream: &[f32], offset: f32) -> Vec<f32> {
    if offset.abs() < 1e-9 {
        return stream.to_vec();
    }
    let factor = 1.0 + offset;
    stream
        .iter()
        .map(|&s| if s > 0.0 { s * factor } else { 0.0 })
        .collect()
}

/// Score a list of (step, RhythmClass) emissions against the labelled
/// segments of a synthetic recording. Returns the confusion matrix.
fn score_emissions(
    rec: &SyntheticRecording,
    emissions: &[(usize, RhythmClass)],
    warmup: usize,
) -> ConfusionMatrix4 {
    let mut cm = ConfusionMatrix4::new();
    let mut current_truth: Option<RhythmClass> = None;
    let mut seen_in_segment = 0usize;
    for &(step, pred) in emissions {
        let truth = match rec.label_for_step(step) {
            Some(t) => t,
            None => continue,
        };
        if Some(truth) != current_truth {
            current_truth = Some(truth);
            seen_in_segment = 0;
        }
        seen_in_segment += 1;
        if seen_in_segment <= warmup {
            continue;
        }
        cm.record(truth, pred);
    }
    cm
}

fn cricketbrain_emit(rec: &SyntheticRecording, drifted: &[f32]) -> Vec<(usize, RhythmClass)> {
    // Run the detector over the drifted stream directly and pair each
    // emission with the labelled segment it lands in. We reuse the
    // synthetic recording's segments for ground truth — those don't
    // change with carrier drift.
    let mut det = CardiacDetector::new();
    det.reset();
    let mut out = Vec::new();
    for (i, &freq) in drifted.iter().enumerate() {
        if let Some(class) = det.step(freq) {
            out.push((i, class));
        }
    }
    let _ = rec; // unused but kept for symmetry with score_emissions
    out
}

fn rule_emit(rule_preds: Vec<BaselinePrediction>) -> Vec<(usize, RhythmClass)> {
    rule_preds.into_iter().map(|p| (p.step, p.class)).collect()
}

fn print_row(off: f32, system: &str, agg: &AggregateMetrics, cm: &ConfusionMatrix4) {
    print!(
        "  off={:+5.1}%  {:<22} total={:>4}  acc={:.3}  macroF1={:.3}",
        off * 100.0,
        system,
        agg.total,
        agg.accuracy,
        agg.macro_f1
    );
    for i in 0..NUM_CLASSES {
        let c = class_from_index(i);
        if cm.support(c) > 0 {
            print!("  R[{}]={:.2}", class_label(c), cm.recall(c));
        }
    }
    println!();
}

fn csv_header() -> String {
    let mut h = String::from(
        "carrier_offset_pct,system,total,correct,accuracy,macro_f1,weighted_f1,balanced_accuracy",
    );
    for i in 0..NUM_CLASSES {
        h.push_str(",recall_");
        h.push_str(class_label(class_from_index(i)));
    }
    h.push('\n');
    h
}

fn csv_row(off: f32, system: &str, agg: &AggregateMetrics, cm: &ConfusionMatrix4) -> String {
    let mut out = format!(
        "{:.4},{},{},{},{:.6},{:.6},{:.6},{:.6}",
        off * 100.0,
        system,
        agg.total,
        agg.correct,
        agg.accuracy,
        agg.macro_f1,
        agg.weighted_f1,
        agg.balanced_accuracy,
    );
    for i in 0..NUM_CLASSES {
        out.push(',');
        out.push_str(&format!("{:.6}", cm.recall(class_from_index(i))));
    }
    out.push('\n');
    out
}

fn main() {
    let args = Args::parse();
    println!("== Cardiac Drift Sweep — CricketBrain Gaussian tuning vs rule's hard band-pass ==");
    println!(
        "  seed={}  beats/class={}  warmup={}  write={}",
        args.seed, args.beats_per_class, args.warmup, args.write
    );

    // Build one labelled synthetic recording. We freeze it once so all
    // drift sweeps see the same underlying patient data.
    let cfg = SyntheticConfig::default()
        .with_seed(args.seed)
        .with_beats_per_class(args.beats_per_class)
        .with_irregular(true);
    let rec = generate(&cfg);
    println!(
        "  generated stream: {} samples, {} segments\n",
        rec.stream.len(),
        rec.segments.len()
    );

    let offsets = [
        -0.20, -0.15, -0.10, -0.07, -0.05, -0.03, -0.01, 0.0, 0.01, 0.03, 0.05, 0.07, 0.10, 0.15,
        0.20,
    ];

    let mut csv = csv_header();

    // For the rule baselines, we re-evaluate `score_emissions` against
    // the same labelled recording.
    let baseline_warmup = args.warmup;

    // Baseline: a sanity check at off=0
    let sanity_run = || {
        let det_emissions = cricketbrain_emit(&rec, &rec.stream);
        let cm = score_emissions(&rec, &det_emissions, baseline_warmup);
        AggregateMetrics::from_cm(&cm).accuracy
    };
    println!("  sanity (CricketBrain @ off=0): {:.3}\n", sanity_run());

    for &off in &offsets {
        let drifted = apply_carrier_drift(&rec.stream, off);

        // CricketBrain
        let det_emit = cricketbrain_emit(&rec, &drifted);
        let cb_emissions: Vec<(usize, RhythmClass)> = {
            // Reuse the same scoring path as the in-tree evaluate helper,
            // but the recording-vs-stream pairing is by step index here.
            // Drop a small per-segment warmup so transitions don't
            // dominate.
            let scored: Vec<ScoredEmission> = det_emit
                .iter()
                .filter_map(|&(step, pred)| {
                    rec.label_for_step(step).map(|truth| ScoredEmission {
                        step,
                        truth,
                        pred,
                        confidence: 1.0,
                        bpm: 0.0,
                    })
                })
                .collect();
            drop_warmup(&scored, baseline_warmup)
                .into_iter()
                .map(|s| (s.step, s.pred))
                .collect()
        };
        let cb_cm = score_emissions(&rec, &cb_emissions, 0);
        let cb_agg = AggregateMetrics::from_cm(&cb_cm);
        print_row(off, "CricketBrain", &cb_agg, &cb_cm);
        csv.push_str(&csv_row(off, "CricketBrain", &cb_agg, &cb_cm));

        // ThresholdBurst rule (hard band-pass at 4500 Hz ±10 %)
        let tb_preds = ThresholdBurstBaseline::default().run(&drifted);
        let tb_emit = rule_emit(tb_preds);
        let tb_cm = score_emissions(&rec, &tb_emit, baseline_warmup);
        let tb_agg = AggregateMetrics::from_cm(&tb_cm);
        print_row(off, "ThresholdBurst-rule", &tb_agg, &tb_cm);
        csv.push_str(&csv_row(off, "ThresholdBurst-rule", &tb_agg, &tb_cm));

        // FrequencyRule baseline
        let fr_preds = FrequencyRuleBaseline::default().run(&drifted);
        let fr_emit = rule_emit(fr_preds);
        let fr_cm = score_emissions(&rec, &fr_emit, baseline_warmup);
        let fr_agg = AggregateMetrics::from_cm(&fr_cm);
        print_row(off, "FrequencyRule", &fr_agg, &fr_cm);
        csv.push_str(&csv_row(off, "FrequencyRule", &fr_agg, &fr_cm));

        println!();
    }

    if !args.write {
        println!("  (--write not passed → CSV not regenerated)");
        return;
    }

    let cmd = current_command_line();
    let mut meta = RunMetadata::new(
        &cmd,
        "synthetic_drift",
        &format!("uc01_drift_seed{}_bpc{}", args.seed, args.beats_per_class),
        args.seed,
        &["Normal", "Tachy", "Brady", "Irregular"],
        1,
        1000,
    );
    meta.limitations = vec![
        "Synthetic recording with deterministic carrier-frequency drift applied to all \
         non-silent samples."
            .into(),
        "Probes ONLY carrier-frequency robustness — not amplitude noise, not morphology drift, \
         not RR variability."
            .into(),
        format!(
            "Nominal QRS carrier: {} Hz. ThresholdBurstBaseline default band: \u{00b1}10 % \
             of carrier. CricketBrain default `min_freq=4000, max_freq=5000` Hz.",
            NOMINAL_QRS_HZ
        ),
        "Not a medical device.".into(),
    ];
    let path = PathBuf::from(RESULT_DIR).join("cardiac_drift_sweep.csv");
    write_csv_with_header(&path, &meta, &csv).expect("write csv");
    println!("  wrote {}", path.display());
}
