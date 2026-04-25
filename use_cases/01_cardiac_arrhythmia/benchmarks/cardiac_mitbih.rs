// SPDX-License-Identifier: AGPL-3.0-only
//! **Skeleton** for running the cardiac detector against
//! preprocessed MIT-BIH records.
//!
//! Status: *planned / not yet validated.*
//!
//! Why a skeleton instead of full results?
//!
//! - The MIT-BIH data is not committed and must be downloaded with
//!   `python python/download_mitbih.py`, which requires PhysioNet
//!   account access.
//! - Real-data validation requires careful R-peak extraction and
//!   patient-level (not beat-level) data splits — *this benchmark
//!   does not pretend to do either yet.*
//! - We refuse to publish numbers from a half-done real-data pipeline.
//!
//! What this binary does today:
//!
//! 1. Looks for a CSV at `data/processed/<record>.csv`
//!    (default: `sample_record.csv`).
//! 2. Runs the existing CSV-based classification path
//!    (`CardiacDetector::classify_stream`) and reports the truth-based
//!    confusion matrix that uses the **CSV-encoded BPM** as ground
//!    truth (NOT the detector's own BPM estimate).
//! 3. Refuses to compute "MIT-BIH validated" metrics: the CSV ships
//!    only with synthetic rows today. The output makes that explicit.
//!
//! Reproduce:
//! ```
//! cargo run --release --example cardiac_mitbih -- \
//!     --csv data/processed/sample_record.csv
//! ```

use std::path::PathBuf;

use cricket_brain_cardiac::detector::{CardiacDetector, RhythmClass};
use cricket_brain_cardiac::ecg_signal;
use cricket_brain_cardiac::metrics::{
    class_from_index, class_label, AggregateMetrics, ConfusionMatrix4, NUM_CLASSES,
};
use cricket_brain_cardiac::report::{current_command_line, write_csv_with_header, RunMetadata};

const RESULT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/results");

struct Args {
    csv: String,
    write: bool,
}

impl Args {
    fn parse() -> Self {
        let mut a = Args {
            csv: "data/processed/sample_record.csv".into(),
            write: false,
        };
        let mut iter = std::env::args().skip(1);
        while let Some(flag) = iter.next() {
            match flag.as_str() {
                "--csv" => a.csv = iter.next().unwrap_or_default(),
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

fn truth_from_bpm(bpm: f32) -> RhythmClass {
    if bpm > 100.0 {
        RhythmClass::Tachycardia
    } else if bpm < 60.0 {
        RhythmClass::Bradycardia
    } else {
        RhythmClass::NormalSinus
    }
}

fn main() {
    let args = Args::parse();
    println!("== Cardiac MIT-BIH skeleton ==");
    println!("  csv={}", args.csv);
    println!(
        "  STATUS: real-data validation is *planned*; this run only \
         exercises the CSV path.\n"
    );

    let beats = ecg_signal::from_csv(&args.csv);
    if beats.is_empty() {
        eprintln!("ERROR: no beats in {}", args.csv);
        eprintln!(
            "Run `python python/preprocess.py --synthetic` to regenerate \
             the synthetic sample, or download MIT-BIH (see data/SOURCES.md) \
             and run `python python/preprocess.py`."
        );
        std::process::exit(1);
    }
    println!("  loaded {} beats from CSV", beats.len());

    let mut det = CardiacDetector::new();
    let preds = det.classify_stream(&beats);
    println!("  detector emissions: {}", preds.len());

    // Build the confusion matrix using the **CSV-encoded BPM** as
    // ground truth (the canonical RR for that beat) — never the
    // detector's own BPM estimate.
    let mut cm = ConfusionMatrix4::new();
    for (i, p) in preds.iter().enumerate() {
        // Find the closest beat in the CSV by step → timestamp.
        // Beats are dense, so a linear sweep is fine here.
        let pred_step_ms = p.step as f32;
        let mut best = 0usize;
        let mut best_d = f32::MAX;
        for (k, b) in beats.iter().enumerate() {
            let d = (b.timestamp_ms - pred_step_ms).abs();
            if d < best_d {
                best_d = d;
                best = k;
            }
        }
        let truth_bpm = beats[best].bpm;
        let truth = truth_from_bpm(truth_bpm);
        cm.record(truth, p.rhythm);
        if i < 5 {
            println!(
                "  [{}] pred={} bpm_est={:.0} truth={}({:.0}) at t≈{:.0}ms",
                i,
                p.rhythm,
                p.bpm,
                class_label(truth),
                truth_bpm,
                pred_step_ms,
            );
        }
    }

    cm.print("Confusion matrix vs CSV-encoded BPM truth:");
    let agg = AggregateMetrics::from_cm(&cm);

    println!(
        "\n  Note: this CSV ships with **synthetic** rows by default. \
         Numbers above are NOT MIT-BIH numbers. To run on MIT-BIH, see \
         `python/download_mitbih.py` and `python/preprocess.py`, then \
         pass `--csv data/processed/<record>.csv`."
    );

    if !args.write {
        println!("\n  (--write not passed → CSV not regenerated)");
        return;
    }

    let cmd = current_command_line();
    let meta = RunMetadata::new(
        &cmd,
        "csv",
        &args.csv,
        0,
        &["Normal", "Tachy", "Brady", "Irregular"],
        1,
        1000,
    );
    let path = PathBuf::from(RESULT_DIR).join("cardiac_csv_summary.csv");
    let mut body = String::from("metric,value\n");
    body.push_str(&format!("total,{}\n", agg.total));
    body.push_str(&format!("correct,{}\n", agg.correct));
    body.push_str(&format!("accuracy,{:.6}\n", agg.accuracy));
    body.push_str(&format!("macro_f1,{:.6}\n", agg.macro_f1));
    body.push_str(&format!("balanced_accuracy,{:.6}\n", agg.balanced_accuracy));
    for i in 0..NUM_CLASSES {
        let c = class_from_index(i);
        if cm.support(c) > 0 {
            body.push_str(&format!("recall_{},{:.6}\n", class_label(c), cm.recall(c)));
            body.push_str(&format!(
                "precision_{},{:.6}\n",
                class_label(c),
                cm.precision(c)
            ));
        }
    }
    write_csv_with_header(&path, &meta, &body).expect("write csv");
    println!("  wrote {}", path.display());
}
