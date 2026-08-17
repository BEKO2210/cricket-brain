// SPDX-License-Identifier: AGPL-3.0-only
//! Compare `CardiacDetector` against simple non-neuromorphic baselines
//! on the same labelled synthetic recording.
//!
//! Results that beat both baselines justify CricketBrain's complexity.
//! Results that are tied with or worse than a baseline are honest
//! evidence the current detector hyperparameters are not yet
//! competitive on that scenario.
//!
//! Reproduce:
//! ```
//! cargo run --release --example cardiac_baselines -- --seed 42 --write
//! ```

use std::path::PathBuf;

use cricket_brain_cardiac::baselines::{
    BaselinePrediction, FrequencyRuleBaseline, ThresholdBurstBaseline,
};
use cricket_brain_cardiac::detector::CardiacDetector;
use cricket_brain_cardiac::evaluate::{drop_warmup, run_and_score, ScoredEmission};
use cricket_brain_cardiac::metrics::{
    class_from_index, class_label, AggregateMetrics, ConfusionMatrix4, NUM_CLASSES,
};
use cricket_brain_cardiac::report::{current_command_line, write_csv_with_header, RunMetadata};
use cricket_brain_cardiac::synthetic::{generate, SyntheticConfig, SyntheticRecording};

const RESULT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/results");

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
            beats_per_class: 30,
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

fn baseline_to_scored(
    rec: &SyntheticRecording,
    preds: &[BaselinePrediction],
) -> Vec<ScoredEmission> {
    let mut out = Vec::with_capacity(preds.len());
    for p in preds {
        if let Some(truth) = rec.label_for_step(p.step) {
            out.push(ScoredEmission {
                step: p.step,
                truth,
                pred: p.class,
                confidence: 1.0,
                bpm: p.bpm,
            });
        }
    }
    out
}

fn confusion_for(scored: &[ScoredEmission]) -> ConfusionMatrix4 {
    let mut cm = ConfusionMatrix4::new();
    for s in scored {
        cm.record(s.truth, s.pred);
    }
    cm
}

fn drop_baseline_warmup(scored: &[ScoredEmission], warmup: usize) -> Vec<ScoredEmission> {
    drop_warmup(scored, warmup)
}

struct ScenarioResult {
    name: &'static str,
    cricketbrain: AggregateMetrics,
    threshold_rule: AggregateMetrics,
    freq_rule: AggregateMetrics,
    cm_cb: ConfusionMatrix4,
    cm_tr: ConfusionMatrix4,
    cm_fr: ConfusionMatrix4,
}

fn evaluate_scenario(name: &'static str, cfg: &SyntheticConfig, args: &Args) -> ScenarioResult {
    let rec = generate(cfg);

    let mut det = CardiacDetector::new();
    let scored_cb = run_and_score(&mut det, &rec);
    let scored_cb = drop_warmup(&scored_cb, args.warmup);
    let cm_cb = confusion_for(&scored_cb);

    let preds_tr = ThresholdBurstBaseline::default().run(&rec.stream);
    let scored_tr = baseline_to_scored(&rec, &preds_tr);
    let scored_tr = drop_baseline_warmup(&scored_tr, args.warmup);
    let cm_tr = confusion_for(&scored_tr);

    let preds_fr = FrequencyRuleBaseline::default().run(&rec.stream);
    let scored_fr = baseline_to_scored(&rec, &preds_fr);
    let scored_fr = drop_baseline_warmup(&scored_fr, args.warmup);
    let cm_fr = confusion_for(&scored_fr);

    ScenarioResult {
        name,
        cricketbrain: AggregateMetrics::from_cm(&cm_cb),
        threshold_rule: AggregateMetrics::from_cm(&cm_tr),
        freq_rule: AggregateMetrics::from_cm(&cm_fr),
        cm_cb,
        cm_tr,
        cm_fr,
    }
}

fn print_scenario(r: &ScenarioResult) {
    println!("\n=== Scenario: {} ===", r.name);
    println!(
        "  {:<20} {:>8} {:>10} {:>10} {:>12}",
        "system", "total", "accuracy", "macro_F1", "balanced_acc"
    );
    println!(
        "  {:<20} {:>8} {:>10.4} {:>10.4} {:>12.4}",
        "CricketBrain",
        r.cricketbrain.total,
        r.cricketbrain.accuracy,
        r.cricketbrain.macro_f1,
        r.cricketbrain.balanced_accuracy,
    );
    println!(
        "  {:<20} {:>8} {:>10.4} {:>10.4} {:>12.4}",
        "ThresholdBurst-rule",
        r.threshold_rule.total,
        r.threshold_rule.accuracy,
        r.threshold_rule.macro_f1,
        r.threshold_rule.balanced_accuracy,
    );
    println!(
        "  {:<20} {:>8} {:>10.4} {:>10.4} {:>12.4}",
        "Frequency-rule",
        r.freq_rule.total,
        r.freq_rule.accuracy,
        r.freq_rule.macro_f1,
        r.freq_rule.balanced_accuracy,
    );
}

fn csv_header() -> String {
    let mut h = String::from(
        "scenario,system,total,correct,accuracy,macro_f1,weighted_f1,balanced_accuracy",
    );
    for i in 0..NUM_CLASSES {
        h.push_str(",recall_");
        h.push_str(class_label(class_from_index(i)));
    }
    h.push('\n');
    h
}

fn csv_row(scenario: &str, system: &str, agg: &AggregateMetrics, cm: &ConfusionMatrix4) -> String {
    let mut out = format!(
        "{},{},{},{},{:.6},{:.6},{:.6},{:.6}",
        scenario,
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
    println!("== Cardiac Baselines vs CricketBrain ==");
    println!(
        "  seed={}  beats/class={}  warmup={}  write={}",
        args.seed, args.beats_per_class, args.warmup, args.write,
    );

    let scenarios: Vec<(&str, SyntheticConfig)> = vec![
        (
            "clean",
            SyntheticConfig::default()
                .with_seed(args.seed)
                .with_beats_per_class(args.beats_per_class)
                .with_irregular(true),
        ),
        (
            "noise_2pct",
            SyntheticConfig::default()
                .with_seed(args.seed)
                .with_beats_per_class(args.beats_per_class)
                .with_irregular(true)
                .with_noise(0.02),
        ),
        (
            "wander_200hz",
            SyntheticConfig::default()
                .with_seed(args.seed)
                .with_beats_per_class(args.beats_per_class)
                .with_irregular(true)
                .with_baseline_wander(200.0),
        ),
        (
            "hrv_10pct",
            SyntheticConfig::default()
                .with_seed(args.seed)
                .with_beats_per_class(args.beats_per_class)
                .with_irregular(true)
                .with_hrv(0.10),
        ),
        (
            "missing_qrs_10pct",
            SyntheticConfig::default()
                .with_seed(args.seed)
                .with_beats_per_class(args.beats_per_class)
                .with_irregular(true)
                .with_missing_qrs(0.10),
        ),
    ];

    let mut all = Vec::new();
    let mut csv = csv_header();
    for (name, cfg) in scenarios {
        let r = evaluate_scenario(name, &cfg, &args);
        print_scenario(&r);
        csv.push_str(&csv_row(r.name, "CricketBrain", &r.cricketbrain, &r.cm_cb));
        csv.push_str(&csv_row(
            r.name,
            "ThresholdBurst",
            &r.threshold_rule,
            &r.cm_tr,
        ));
        csv.push_str(&csv_row(r.name, "FrequencyRule", &r.freq_rule, &r.cm_fr));
        all.push(r);
    }

    if !args.write {
        println!("\n  (--write not passed → CSV not regenerated)");
        return;
    }

    let cmd = current_command_line();
    let meta = RunMetadata::new(
        &cmd,
        "synthetic",
        &format!(
            "uc01_synth_v0.2_seed{}_bpc{}",
            args.seed, args.beats_per_class
        ),
        args.seed,
        &["Normal", "Tachy", "Brady", "Irregular"],
        1,
        1000,
    );
    let path = PathBuf::from(RESULT_DIR).join("cardiac_baselines.csv");
    write_csv_with_header(&path, &meta, &csv).expect("write csv");
    println!("\n  wrote {}", path.display());
    let _ = all;
}
