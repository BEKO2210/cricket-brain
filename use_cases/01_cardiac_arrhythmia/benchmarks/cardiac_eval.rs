// SPDX-License-Identifier: AGPL-3.0-only
//! Proper truth-based evaluation of `CardiacDetector`.
//!
//! Unlike the legacy `--csv` confusion-matrix path, this benchmark
//! generates a **labelled synthetic recording** with explicit
//! ground-truth segments and pairs every detector emission with the
//! ground-truth label of the segment in which it was emitted.
//!
//! Reproduce:
//! ```
//! cargo run --release --example cardiac_eval -- \
//!     --seed 42 --beats-per-class 30
//! ```
//!
//! Outputs (only when `--write` is passed):
//!
//! - `results/cardiac_synthetic_summary.json`
//! - `results/cardiac_confusion_matrix.csv`
//! - `results/cardiac_per_class_metrics.csv`

use std::path::PathBuf;

use cricket_brain_cardiac::detector::CardiacDetector;
use cricket_brain_cardiac::evaluate::{confusion_matrix, drop_warmup, run_and_score};
use cricket_brain_cardiac::metrics::{
    class_from_index, class_label, AggregateMetrics, ConfusionMatrix4, PerClassMetrics, NUM_CLASSES,
};
use cricket_brain_cardiac::report::{
    current_command_line, json_array, json_f64, json_object, write_csv_with_header, JsonReport,
    RunMetadata,
};
use cricket_brain_cardiac::synthetic::{generate, SyntheticConfig};

const RESULT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/results");

struct Args {
    seed: u64,
    beats_per_class: u32,
    include_irregular: bool,
    warmup: usize,
    use_preprocessor: bool,
    write: bool,
}

impl Args {
    fn parse() -> Self {
        let mut a = Args {
            seed: 42,
            beats_per_class: 30,
            include_irregular: true,
            warmup: 2,
            use_preprocessor: false,
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
                "--no-irregular" => a.include_irregular = false,
                "--warmup" => {
                    a.warmup = iter.next().unwrap_or_default().parse().unwrap_or(a.warmup)
                }
                "--preprocessor" => a.use_preprocessor = true,
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

fn render_per_class_csv(cm: &ConfusionMatrix4) -> String {
    let mut out = String::from(PerClassMetrics::csv_header());
    out.push('\n');
    for i in 0..NUM_CLASSES {
        let pcm = PerClassMetrics::from_cm(cm, class_from_index(i));
        if pcm.support == 0 {
            continue;
        }
        out.push_str(&pcm.to_csv_row());
        out.push('\n');
    }
    out
}

fn per_class_json_array(cm: &ConfusionMatrix4) -> String {
    let mut blocks: Vec<String> = Vec::new();
    for i in 0..NUM_CLASSES {
        let pcm = PerClassMetrics::from_cm(cm, class_from_index(i));
        if pcm.support == 0 {
            continue;
        }
        blocks.push(json_object(&[
            ("class", format!("\"{}\"", class_label(pcm.class))),
            ("support", pcm.support.to_string()),
            ("tp", pcm.tp.to_string()),
            ("fp", pcm.fp.to_string()),
            ("fn", pcm.fn_.to_string()),
            ("tn", pcm.tn.to_string()),
            ("precision", json_f64(pcm.precision)),
            ("recall", json_f64(pcm.recall)),
            ("specificity", json_f64(pcm.specificity)),
            ("f1", json_f64(pcm.f1)),
        ]));
    }
    json_array(&blocks)
}

fn aggregate_json(agg: &AggregateMetrics) -> String {
    json_object(&[
        ("total", agg.total.to_string()),
        ("correct", agg.correct.to_string()),
        ("accuracy", json_f64(agg.accuracy)),
        ("macro_f1", json_f64(agg.macro_f1)),
        ("weighted_f1", json_f64(agg.weighted_f1)),
        ("balanced_accuracy", json_f64(agg.balanced_accuracy)),
    ])
}

fn confusion_json(cm: &ConfusionMatrix4) -> String {
    // Render rows manually because we need dynamic keys ("pred_<Class>").
    let mut rows: Vec<String> = Vec::new();
    for t in 0..NUM_CLASSES {
        let truth = class_from_index(t);
        let mut row = String::from("{");
        row.push_str(&format!("\"truth\": \"{}\"", class_label(truth)));
        for p in 0..NUM_CLASSES {
            let pred = class_from_index(p);
            row.push_str(&format!(", \"pred_{}\": {}", class_label(pred), cm.m[t][p]));
        }
        row.push('}');
        rows.push(row);
    }
    json_array(&rows)
}

fn report_per_class_table(cm: &ConfusionMatrix4) {
    println!("\n  Per-class metrics:");
    println!(
        "  {:<10} {:>8} {:>8} {:>8} {:>8} {:>10} {:>10} {:>10}",
        "class", "support", "tp", "fp", "fn", "precision", "recall", "f1"
    );
    for i in 0..NUM_CLASSES {
        let pcm = PerClassMetrics::from_cm(cm, class_from_index(i));
        if pcm.support == 0 {
            continue;
        }
        println!(
            "  {:<10} {:>8} {:>8} {:>8} {:>8} {:>10.4} {:>10.4} {:>10.4}",
            class_label(pcm.class),
            pcm.support,
            pcm.tp,
            pcm.fp,
            pcm.fn_,
            pcm.precision,
            pcm.recall,
            pcm.f1,
        );
    }
}

fn main() {
    let args = Args::parse();
    println!("== Cardiac Eval — proper truth-based metrics ==");
    println!(
        "  seed={}  beats/class={}  irregular={}  warmup={}  preprocessor={}  write={}",
        args.seed,
        args.beats_per_class,
        args.include_irregular,
        args.warmup,
        args.use_preprocessor,
        args.write,
    );

    let cfg = SyntheticConfig::default()
        .with_seed(args.seed)
        .with_beats_per_class(args.beats_per_class)
        .with_irregular(args.include_irregular);
    let rec = generate(&cfg);
    println!(
        "  generated stream: {} samples, {} segments",
        rec.stream.len(),
        rec.segments.len()
    );

    let mut det = if args.use_preprocessor {
        CardiacDetector::with_preprocessor(true)
    } else {
        CardiacDetector::new()
    };
    let scored = run_and_score(&mut det, &rec);
    let scored_after_warmup = drop_warmup(&scored, args.warmup);

    let cm_full = confusion_matrix(&scored);
    let cm_warm = confusion_matrix(&scored_after_warmup);

    let agg_full = AggregateMetrics::from_cm(&cm_full);
    let agg_warm = AggregateMetrics::from_cm(&cm_warm);

    cm_full.print("Confusion matrix (all emissions):");
    println!("  → emissions: {}", scored.len());
    report_per_class_table(&cm_full);

    cm_warm.print(&format!(
        "Confusion matrix (after {}-emission per-segment warmup):",
        args.warmup
    ));
    println!("  → emissions: {}", scored_after_warmup.len());
    report_per_class_table(&cm_warm);

    println!(
        "\n  Detector RAM: {} bytes  | classes={:?}",
        det.memory_usage_bytes(),
        ["Normal", "Tachy", "Brady", "Irregular"]
    );

    if !args.write {
        println!("\n  (--write not passed → result files not regenerated)");
        return;
    }

    let cmd = current_command_line();
    let dataset_name = format!(
        "uc01_synth_v0.2_seed{}_bpc{}",
        args.seed, args.beats_per_class
    );
    let classes: Vec<&str> = if args.include_irregular {
        vec!["Normal", "Tachy", "Brady", "Irregular"]
    } else {
        vec!["Normal", "Tachy", "Brady"]
    };
    let meta = RunMetadata::new(
        &cmd,
        "synthetic",
        &dataset_name,
        args.seed,
        &classes,
        1,
        1000,
    );

    // ---------------------- JSON ----------------------
    let mut report = JsonReport::new(meta.clone());
    report
        .add_block(
            "config",
            json_object(&[
                ("beats_per_class", args.beats_per_class.to_string()),
                ("include_irregular", args.include_irregular.to_string()),
                ("warmup_emissions_per_segment", args.warmup.to_string()),
                ("preprocessor_enabled", args.use_preprocessor.to_string()),
                ("detector_ram_bytes", det.memory_usage_bytes().to_string()),
            ]),
        )
        .add_block("aggregate_full", aggregate_json(&agg_full))
        .add_block("aggregate_after_warmup", aggregate_json(&agg_warm))
        .add_block("per_class_full", per_class_json_array(&cm_full))
        .add_block("per_class_after_warmup", per_class_json_array(&cm_warm))
        .add_block("confusion_matrix_full", confusion_json(&cm_full))
        .add_block("confusion_matrix_after_warmup", confusion_json(&cm_warm));

    let json_path = PathBuf::from(RESULT_DIR).join("cardiac_synthetic_summary.json");
    report.write_to(&json_path).expect("write JSON");
    println!("  wrote {}", json_path.display());

    // ---------------------- CSV: confusion matrix ----------------------
    let cm_csv_path = PathBuf::from(RESULT_DIR).join("cardiac_confusion_matrix.csv");
    write_csv_with_header(&cm_csv_path, &meta, &cm_full.to_csv()).expect("write CM csv");
    println!("  wrote {}", cm_csv_path.display());

    // ---------------------- CSV: per-class metrics ----------------------
    let pc_csv_path = PathBuf::from(RESULT_DIR).join("cardiac_per_class_metrics.csv");
    write_csv_with_header(&pc_csv_path, &meta, &render_per_class_csv(&cm_full))
        .expect("write per-class csv");
    println!("  wrote {}", pc_csv_path.display());

    // ---------------------- Markdown: failure cases ----------------------
    let mut md = String::new();
    md.push_str("# Cardiac UC01 — auto-captured failure cases\n\n");
    md.push_str(&format!(
        "Generated: {} | seed: {} | dataset: `{}`\n\n",
        meta.generated_at, meta.seed, meta.dataset_name
    ));
    md.push_str(
        "Each row is one detector emission whose **prediction differed from the ground-truth segment label**. \
         The detector reports a BPM estimate at emission time; the ground-truth column is taken from the labelled \
         synthetic segment in which the emission occurred.\n\n",
    );
    md.push_str("| step | truth | predicted | bpm | confidence |\n");
    md.push_str("|-----:|:------|:----------|----:|-----------:|\n");
    let mut shown = 0;
    for s in &scored_after_warmup {
        if s.truth != s.pred {
            md.push_str(&format!(
                "| {} | {} | {} | {:.0} | {:.2} |\n",
                s.step,
                class_label(s.truth),
                class_label(s.pred),
                s.bpm,
                s.confidence,
            ));
            shown += 1;
            if shown >= 200 {
                break;
            }
        }
    }
    if shown == 0 {
        md.push_str("| — | — | — | — | — |\n\n_All emissions correct after warmup._\n");
    }
    let fc_path = PathBuf::from(RESULT_DIR).join("cardiac_failure_cases.md");
    if let Some(parent) = fc_path.parent() {
        std::fs::create_dir_all(parent).expect("create results dir");
    }
    std::fs::write(&fc_path, md).expect("write failures md");
    println!("  wrote {}", fc_path.display());

    // ---------------------- benchmark_config.json ----------------------
    let mut cfg_report = JsonReport::new(meta.clone());
    cfg_report.add_block(
        "synthetic_config",
        json_object(&[
            ("seed", cfg.seed.to_string()),
            ("beats_per_class", cfg.beats_per_class.to_string()),
            ("include_irregular", cfg.include_irregular.to_string()),
            ("hrv", json_f64(cfg.hrv as f64)),
            ("noise_prob", json_f64(cfg.noise_prob as f64)),
            ("amp_jitter", json_f64(cfg.amp_jitter as f64)),
            (
                "baseline_wander_hz",
                json_f64(cfg.baseline_wander_hz as f64),
            ),
            ("morph_jitter", json_f64(cfg.morph_jitter as f64)),
            ("missing_qrs_prob", json_f64(cfg.missing_qrs_prob as f64)),
            ("motion_burst_prob", json_f64(cfg.motion_burst_prob as f64)),
        ]),
    );
    cfg_report.add_block(
        "detector",
        json_object(&[
            ("ram_bytes", det.memory_usage_bytes().to_string()),
            (
                "preprocessor",
                if args.use_preprocessor {
                    "\"on\""
                } else {
                    "\"off\""
                }
                .to_string(),
            ),
        ]),
    );
    let cfg_path = PathBuf::from(RESULT_DIR).join("benchmark_config.json");
    cfg_report.write_to(&cfg_path).expect("write config");
    println!("  wrote {}", cfg_path.display());
}
