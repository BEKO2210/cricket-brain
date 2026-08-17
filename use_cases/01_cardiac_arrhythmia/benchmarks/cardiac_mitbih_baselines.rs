// SPDX-License-Identifier: AGPL-3.0-only
//! CricketBrain vs rule-based baselines on real MIT-BIH records (v0.5).
//!
//! Same patient-aware loader as `cardiac_mitbih`, same non-circular
//! ground truth (5-beat sliding RR window over annotation intervals),
//! but evaluates three systems side-by-side on each record:
//!
//! 1. **CricketBrain** (`CardiacDetector`) — neuromorphic detector.
//! 2. **ThresholdBurstBaseline** — band-gate + RR-window rule, the
//!    closest non-neuromorphic equivalent to the CricketBrain rule
//!    layer.
//! 3. **FrequencyRuleBaseline** — 1-second-window QRS-burst counter.
//!
//! Reproduce:
//! ```
//! cargo run --release --example cardiac_mitbih_baselines -- \
//!     --records-dir /tmp/mitbih_ds2 --aami-split ds2 --write
//! ```
//!
//! Outputs (when `--write`): `results/cardiac_mitbih_baselines.csv`
//! with one row per (record_id, system).

use std::path::PathBuf;

use cricket_brain_cardiac::baselines::{
    BaselinePrediction, FrequencyRuleBaseline, ThresholdBurstBaseline,
};
use cricket_brain_cardiac::detector::{CardiacDetector, RhythmClass};
use cricket_brain_cardiac::ecg_signal::{from_csv, from_csv_dir, BeatRecord};
use cricket_brain_cardiac::metrics::{
    class_from_index, class_label, AggregateMetrics, ConfusionMatrix4, NUM_CLASSES,
};
use cricket_brain_cardiac::mitbih::{
    rate_regime_truth, rhythm_label_to_class, RateRegimeWindow, AAMI_DS1, AAMI_DS2,
};
use cricket_brain_cardiac::report::{current_command_line, write_csv_with_header, RunMetadata};

const RESULT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/results");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroundTruthMode {
    Rr,
    Annot,
    Hybrid,
}

impl GroundTruthMode {
    fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "rr" => GroundTruthMode::Rr,
            "annot" | "annotation" => GroundTruthMode::Annot,
            "hybrid" => GroundTruthMode::Hybrid,
            _ => {
                eprintln!("ERROR: --ground-truth must be rr|annot|hybrid (got {s})");
                std::process::exit(2);
            }
        }
    }
    fn label(self) -> &'static str {
        match self {
            GroundTruthMode::Rr => "rr",
            GroundTruthMode::Annot => "annot",
            GroundTruthMode::Hybrid => "hybrid",
        }
    }
}

struct Args {
    records_dir: Option<String>,
    csv: Option<String>,
    write: bool,
    warmup: usize,
    aami_split: Option<String>,
    ground_truth: GroundTruthMode,
}

impl Args {
    fn parse() -> Self {
        let mut a = Args {
            records_dir: None,
            csv: None,
            write: false,
            warmup: 2,
            aami_split: None,
            ground_truth: GroundTruthMode::Hybrid,
        };
        let mut iter = std::env::args().skip(1);
        while let Some(flag) = iter.next() {
            match flag.as_str() {
                "--records-dir" => a.records_dir = iter.next(),
                "--csv" => a.csv = iter.next(),
                "--warmup" => {
                    a.warmup = iter.next().unwrap_or_default().parse().unwrap_or(a.warmup)
                }
                "--write" => a.write = true,
                "--aami-split" => a.aami_split = iter.next(),
                "--ground-truth" => {
                    a.ground_truth = GroundTruthMode::from_str(&iter.next().unwrap_or_default())
                }
                _ => {
                    eprintln!("Unknown flag: {flag}");
                    std::process::exit(2);
                }
            }
        }
        a
    }
}

fn load_records(args: &Args) -> Vec<(String, Vec<BeatRecord>)> {
    if let Some(dir) = &args.records_dir {
        return from_csv_dir(dir);
    }
    if let Some(path) = &args.csv {
        let beats = from_csv(path);
        let mut by_id: std::collections::BTreeMap<String, Vec<BeatRecord>> =
            std::collections::BTreeMap::new();
        for b in beats {
            by_id.entry(b.record_id.clone()).or_default().push(b);
        }
        return by_id.into_iter().collect();
    }
    let default =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/processed/sample_record.csv");
    let beats = from_csv(default.to_str().unwrap_or(""));
    let mut by_id: std::collections::BTreeMap<String, Vec<BeatRecord>> =
        std::collections::BTreeMap::new();
    for b in beats {
        by_id.entry(b.record_id.clone()).or_default().push(b);
    }
    by_id.into_iter().collect()
}

fn apply_aami_filter(
    records: Vec<(String, Vec<BeatRecord>)>,
    split: &str,
) -> Vec<(String, Vec<BeatRecord>)> {
    let allow: &[&str] = match split {
        "ds1" | "DS1" => AAMI_DS1,
        "ds2" | "DS2" => AAMI_DS2,
        _ => {
            eprintln!("ERROR: --aami-split must be ds1 or ds2 (got {split})");
            std::process::exit(2);
        }
    };
    records
        .into_iter()
        .filter(|(id, _)| allow.contains(&id.as_str()))
        .collect()
}

/// One scored emission for a generic system.
struct Scored {
    step: usize,
    pred: RhythmClass,
}

/// Run CricketBrain over a beat list and emit (step, pred) tuples.
fn cricketbrain_run(beats: &[BeatRecord]) -> Vec<Scored> {
    let mut det = CardiacDetector::new();
    det.classify_stream(beats)
        .into_iter()
        .map(|p| Scored {
            step: p.step,
            pred: p.rhythm,
        })
        .collect()
}

/// Run a rule baseline (already returns BaselinePrediction).
fn baseline_run(preds: Vec<BaselinePrediction>) -> Vec<Scored> {
    preds
        .into_iter()
        .map(|p| Scored {
            step: p.step,
            pred: p.class,
        })
        .collect()
}

/// Score a system's emissions against the per-beat ground truth.
fn score_system(
    beats: &[BeatRecord],
    emissions: &[Scored],
    win: &RateRegimeWindow,
    warmup: usize,
    gt: GroundTruthMode,
) -> ConfusionMatrix4 {
    // Pre-compute per-beat ground truth + cumulative end steps.
    let rr_ms: Vec<u32> = beats
        .iter()
        .map(|b| b.rr_interval_ms.max(1.0) as u32)
        .collect();
    let truth: Vec<Option<RhythmClass>> = (0..beats.len())
        .map(|i| match gt {
            GroundTruthMode::Rr => rate_regime_truth(&rr_ms, i, win),
            GroundTruthMode::Annot => rhythm_label_to_class(&beats[i].rhythm_label),
            GroundTruthMode::Hybrid => rhythm_label_to_class(&beats[i].rhythm_label)
                .or_else(|| rate_regime_truth(&rr_ms, i, win)),
        })
        .collect();

    let mut beat_end_step: Vec<usize> = Vec::with_capacity(beats.len());
    let mut acc: usize = 0;
    for b in beats {
        acc = acc.saturating_add(b.rr_interval_ms as usize);
        beat_end_step.push(acc);
    }

    let mut cm = ConfusionMatrix4::new();
    let mut last_truth: Option<RhythmClass> = None;
    let mut warmup_skipped: usize = 0;
    for s in emissions {
        let beat_idx = beat_end_step.partition_point(|&end| end < s.step);
        if beat_idx >= beats.len() {
            continue;
        }
        let t = match truth[beat_idx] {
            Some(t) => t,
            None => continue,
        };
        if Some(t) != last_truth {
            last_truth = Some(t);
            warmup_skipped = 0;
        }
        warmup_skipped += 1;
        if warmup_skipped <= warmup {
            continue;
        }
        cm.record(t, s.pred);
    }
    cm
}

fn print_row(record_id: &str, system: &str, agg: &AggregateMetrics, cm: &ConfusionMatrix4) {
    println!(
        "  {:<8} {:<22} total={:>5} acc={:.4} macroF1={:.4} balAcc={:.4}  R[N]={:.2} R[T]={:.2} R[B]={:.2} R[I]={:.2}",
        record_id,
        system,
        agg.total,
        agg.accuracy,
        agg.macro_f1,
        agg.balanced_accuracy,
        cm.recall(class_from_index(0)),
        cm.recall(class_from_index(1)),
        cm.recall(class_from_index(2)),
        cm.recall(class_from_index(3)),
    );
}

fn csv_header() -> String {
    let mut h = String::from(
        "record_id,system,total,correct,accuracy,macro_f1,weighted_f1,balanced_accuracy",
    );
    for i in 0..NUM_CLASSES {
        h.push_str(",recall_");
        h.push_str(class_label(class_from_index(i)));
    }
    for i in 0..NUM_CLASSES {
        h.push_str(",precision_");
        h.push_str(class_label(class_from_index(i)));
    }
    h.push('\n');
    h
}

fn csv_row(record_id: &str, system: &str, agg: &AggregateMetrics, cm: &ConfusionMatrix4) -> String {
    let mut out = format!(
        "{},{},{},{},{:.6},{:.6},{:.6},{:.6}",
        record_id,
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
    for i in 0..NUM_CLASSES {
        out.push(',');
        out.push_str(&format!("{:.6}", cm.precision(class_from_index(i))));
    }
    out.push('\n');
    out
}

fn main() {
    let args = Args::parse();
    println!(
        "== Cardiac MIT-BIH Baselines (v0.6: CricketBrain vs rules on real data) ==  ground_truth={}",
        args.ground_truth.label()
    );

    let mut records = load_records(&args);
    if records.is_empty() {
        eprintln!("ERROR: no beats loaded.");
        std::process::exit(1);
    }
    if let Some(split) = &args.aami_split {
        let before = records.len();
        records = apply_aami_filter(records, split);
        println!(
            "  AAMI filter --aami-split={split} kept {} of {} records",
            records.len(),
            before
        );
    }
    println!(
        "  evaluating {} record(s), {} total beats",
        records.len(),
        records.iter().map(|(_, b)| b.len()).sum::<usize>()
    );

    let win = RateRegimeWindow::default();
    let mut csv = csv_header();
    let mut pooled: std::collections::HashMap<&str, ConfusionMatrix4> =
        std::collections::HashMap::new();

    for (id, beats) in &records {
        // CricketBrain
        let cb_emissions = cricketbrain_run(beats);
        let cb_cm = score_system(beats, &cb_emissions, &win, args.warmup, args.ground_truth);
        let cb_agg = AggregateMetrics::from_cm(&cb_cm);
        print_row(id, "CricketBrain", &cb_agg, &cb_cm);
        csv.push_str(&csv_row(id, "CricketBrain", &cb_agg, &cb_cm));
        let p = pooled.entry("CricketBrain").or_default();
        for t in 0..NUM_CLASSES {
            for q in 0..NUM_CLASSES {
                p.m[t][q] += cb_cm.m[t][q];
            }
        }

        // ThresholdBurst rule
        let stream = cricket_brain_cardiac::ecg_signal::beats_to_frequency_stream(beats);
        let tb_preds = ThresholdBurstBaseline::default().run(&stream);
        let tb_emissions = baseline_run(tb_preds);
        let tb_cm = score_system(beats, &tb_emissions, &win, args.warmup, args.ground_truth);
        let tb_agg = AggregateMetrics::from_cm(&tb_cm);
        print_row(id, "ThresholdBurst-rule", &tb_agg, &tb_cm);
        csv.push_str(&csv_row(id, "ThresholdBurst-rule", &tb_agg, &tb_cm));
        let p = pooled.entry("ThresholdBurst-rule").or_default();
        for t in 0..NUM_CLASSES {
            for q in 0..NUM_CLASSES {
                p.m[t][q] += tb_cm.m[t][q];
            }
        }

        // Frequency rule
        let fr_preds = FrequencyRuleBaseline::default().run(&stream);
        let fr_emissions = baseline_run(fr_preds);
        let fr_cm = score_system(beats, &fr_emissions, &win, args.warmup, args.ground_truth);
        let fr_agg = AggregateMetrics::from_cm(&fr_cm);
        print_row(id, "FrequencyRule", &fr_agg, &fr_cm);
        csv.push_str(&csv_row(id, "FrequencyRule", &fr_agg, &fr_cm));
        let p = pooled.entry("FrequencyRule").or_default();
        for t in 0..NUM_CLASSES {
            for q in 0..NUM_CLASSES {
                p.m[t][q] += fr_cm.m[t][q];
            }
        }
    }

    // -------------------- Pooled summary --------------------
    println!("\n  Pooled across {} record(s):", records.len());
    println!(
        "  {:<22} {:>5}  {:>10}  {:>10}  {:>10}",
        "system", "total", "accuracy", "macro_F1", "balanced_acc"
    );
    let order = ["CricketBrain", "ThresholdBurst-rule", "FrequencyRule"];
    for sys in order {
        if let Some(cm) = pooled.get(sys) {
            let agg = AggregateMetrics::from_cm(cm);
            println!(
                "  {:<22} {:>5}  {:>10.4}  {:>10.4}  {:>10.4}",
                sys, agg.total, agg.accuracy, agg.macro_f1, agg.balanced_accuracy
            );
        }
    }

    // pooled rows in CSV
    csv.push_str(&format!("# pooled across {} record(s)\n", records.len()));
    for sys in order {
        if let Some(cm) = pooled.get(sys) {
            let agg = AggregateMetrics::from_cm(cm);
            csv.push_str(&csv_row("__pooled__", sys, &agg, cm));
        }
    }

    if !args.write {
        println!("\n  (--write not passed → CSV not regenerated)");
        return;
    }

    let cmd = current_command_line();
    let split_tag = args
        .aami_split
        .as_deref()
        .map(|s| format!("_{}", s.to_uppercase()))
        .unwrap_or_default();
    let dataset_name = format!(
        "uc01_mitbih_baselines{}_{}records",
        split_tag,
        records.len()
    );
    let mut meta = RunMetadata::new(
        &cmd,
        "mitbih_csv",
        &dataset_name,
        0,
        &["Normal", "Tachy", "Brady", "Irregular"],
        1,
        1000,
    );
    meta.limitations = vec![
        "Real MIT-BIH records (PhysioNet, ODC-By v1.0).".into(),
        "Rate-regime triage only; ground truth from annotation RR intervals via 5-beat \
         sliding window (mitbih::rate_regime_truth)."
            .into(),
        "Three systems compared on the same emissions: CricketBrain, ThresholdBurstBaseline, \
         FrequencyRuleBaseline."
            .into(),
        match args.aami_split.as_deref() {
            Some(s) => format!(
                "AAMI EC57:2012 record set: {}. CricketBrain has no training phase, so DS1/DS2 \
                 are used purely as canonical record lists.",
                s.to_uppercase()
            ),
            None => "No AAMI split filter applied.".into(),
        },
        "Not a medical device. Research / embedded pre-screening prototype only.".into(),
    ];
    let path = PathBuf::from(RESULT_DIR).join("cardiac_mitbih_baselines.csv");
    write_csv_with_header(&path, &meta, &csv).expect("write csv");
    println!("\n  wrote {}", path.display());
}
