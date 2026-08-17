// SPDX-License-Identifier: AGPL-3.0-only
//! Reject-aware coverage / accuracy curve for `CardiacDetector`.
//!
//! Sweeps a confidence threshold and reports:
//!
//! - **coverage** — fraction of emissions the system commits to
//!   (i.e. confidence ≥ threshold)
//! - **covered_accuracy** — accuracy on the covered subset; this is
//!   what matters when the system is allowed to abstain
//! - **forced_accuracy** — what the system would score if every
//!   reject were treated as wrong (worst-case bound)
//!
//! Reproduce:
//! ```
//! cargo run --release --example cardiac_reject -- --seed 42 --write
//! ```

use std::path::PathBuf;

use cricket_brain_cardiac::detector::CardiacDetector;
use cricket_brain_cardiac::evaluate::{drop_warmup, run_and_score, to_truth_pred_conf};
use cricket_brain_cardiac::metrics::coverage_accuracy_curve;
use cricket_brain_cardiac::report::{current_command_line, write_csv_with_header, RunMetadata};
use cricket_brain_cardiac::synthetic::{generate, SyntheticConfig};

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

fn main() {
    let args = Args::parse();
    println!("== Cardiac Reject Curve ==");
    println!(
        "  seed={}  beats/class={}  warmup={}  write={}",
        args.seed, args.beats_per_class, args.warmup, args.write,
    );

    let cfg = SyntheticConfig::default()
        .with_seed(args.seed)
        .with_beats_per_class(args.beats_per_class)
        .with_irregular(true);
    let rec = generate(&cfg);
    let mut det = CardiacDetector::new();
    let scored = run_and_score(&mut det, &rec);
    let scored = drop_warmup(&scored, args.warmup);
    let samples = to_truth_pred_conf(&scored);
    println!("  emissions: {}", samples.len());

    let thresholds = [
        0.0, 0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80, 0.90, 0.95, 1.00,
    ];
    let curve = coverage_accuracy_curve(&samples, &thresholds);

    println!(
        "  {:>10} {:>10} {:>10} {:>17} {:>15} {:>15}",
        "threshold", "covered", "rejected", "coverage", "covered_acc", "forced_acc"
    );
    for p in &curve {
        println!(
            "  {:>10.2} {:>10} {:>10} {:>17.4} {:>15.4} {:>15.4}",
            p.confidence_threshold,
            p.covered,
            samples.len() as u32 - p.covered,
            p.coverage,
            p.covered_accuracy,
            p.forced_accuracy,
        );
    }

    if !args.write {
        println!("\n  (--write not passed → CSV not regenerated)");
        return;
    }

    let mut csv = String::from(
        "threshold,covered,correct_covered,coverage,covered_accuracy,forced_accuracy\n",
    );
    for p in &curve {
        csv.push_str(&format!(
            "{:.4},{},{},{:.6},{:.6},{:.6}\n",
            p.confidence_threshold,
            p.covered,
            p.correct_covered,
            p.coverage,
            p.covered_accuracy,
            p.forced_accuracy,
        ));
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
    let path = PathBuf::from(RESULT_DIR).join("cardiac_reject_curve.csv");
    write_csv_with_header(&path, &meta, &csv).expect("write csv");
    println!("\n  wrote {}", path.display());
}
