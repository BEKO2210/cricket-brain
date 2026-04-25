// SPDX-License-Identifier: AGPL-3.0-only
//! Patient-level evaluation of `CardiacDetector` against MIT-BIH-style
//! preprocessed records (v0.3).
//!
//! ## What this binary does
//!
//! 1. Scans a directory of `*.csv` files (default
//!    `data/processed/sample_record.csv` plus `data/processed/{train,test}/`)
//!    and groups beats by `record_id` (v0.3 6-col CSV format; legacy 5-col
//!    falls back to file stem).
//! 2. For each record, builds a beat-aligned frequency stream and runs
//!    [`CardiacDetector`] against it. Detector emissions are paired with
//!    the **non-circular** rate-regime ground truth derived from the
//!    raw annotation RR intervals via [`mitbih::rate_regime_truth`].
//! 3. Reports per-record metrics + a macro-pooled summary (each record
//!    weighted equally) so a single long record cannot dominate the
//!    headline number.
//!
//! ## What this binary refuses to do
//!
//! - **No fake "validated" numbers.** If every loaded `record_id`
//!   starts with `synth_`, the bench writes
//!   `results/cardiac_mitbih_skeleton_only.json` with a clear
//!   "no real records found" status instead of producing a
//!   "validated" file.
//! - **No claim of AAMI morphology classification.** The detector is a
//!   rate-regime classifier; AAMI N/S/V/F/Q distributions are reported
//!   for *traceability only*, never as accuracy metrics.
//!
//! ## Reproduce
//!
//! ```bash
//! # Skeleton only (uses ./data/processed which currently only ships
//! # the synthetic sample):
//! cargo run --release --example cardiac_mitbih -- --write
//!
//! # On real MIT-BIH data, after `python python/download_mitbih.py`
//! # and `python python/preprocess.py`:
//! cargo run --release --example cardiac_mitbih -- \
//!     --records-dir use_cases/01_cardiac_arrhythmia/data/processed/test \
//!     --write
//! ```

use std::path::{Path, PathBuf};

use cricket_brain_cardiac::detector::{BeatClassification, CardiacDetector, RhythmClass};
use cricket_brain_cardiac::ecg_signal::{from_csv, from_csv_dir, BeatRecord};
use cricket_brain_cardiac::metrics::{
    class_from_index, class_label, AggregateMetrics, ConfusionMatrix4, NUM_CLASSES,
};
use cricket_brain_cardiac::mitbih::{
    aami_from_symbol, rate_regime_truth, AamiClass, PerRecordResult, PooledResult, RateRegimeWindow,
};
use cricket_brain_cardiac::report::{
    current_command_line, json_array, json_f64, json_object, json_quote, write_csv_with_header,
    JsonReport, RunMetadata,
};

const RESULT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/results");

struct Args {
    records_dir: Option<String>,
    csv: Option<String>,
    write: bool,
    warmup: usize,
}

impl Args {
    fn parse() -> Self {
        let mut a = Args {
            records_dir: None,
            csv: None,
            write: false,
            warmup: 2,
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
                _ => {
                    eprintln!("Unknown flag: {flag}");
                    std::process::exit(2);
                }
            }
        }
        a
    }
}

/// Group all loaded CSV beats by `record_id`. Returns
/// `Vec<(record_id, beats)>` ordered by record_id.
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
    // Default: use the bundled synthetic sample CSV.
    let default = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/processed/sample_record.csv");
    let beats = from_csv(default.to_str().unwrap_or(""));
    let mut by_id: std::collections::BTreeMap<String, Vec<BeatRecord>> =
        std::collections::BTreeMap::new();
    for b in beats {
        by_id.entry(b.record_id.clone()).or_default().push(b);
    }
    by_id.into_iter().collect()
}

/// True if every record_id is "obviously synthetic" (starts with
/// `synth_`). The bench refuses to publish "validated" numbers in
/// that case.
fn all_synthetic(records: &[(String, Vec<BeatRecord>)]) -> bool {
    !records.is_empty() && records.iter().all(|(id, _)| id.starts_with("synth_"))
}

/// Tally AAMI class distribution for a record.
fn aami_counts_for(beats: &[BeatRecord]) -> [u32; 5] {
    let mut counts = [0u32; 5];
    for b in beats {
        match aami_from_symbol(&b.beat_type) {
            Some(AamiClass::Normal) => counts[0] += 1,
            Some(AamiClass::Supraventricular) => counts[1] += 1,
            Some(AamiClass::Ventricular) => counts[2] += 1,
            Some(AamiClass::Fusion) => counts[3] += 1,
            Some(AamiClass::Unknown) => counts[4] += 1,
            None => {} // non-beat annotation, ignore
        }
    }
    counts
}

/// Run the detector on one record's beat list and return
/// (per-record stats, confusion matrix, list of failures).
struct RecordEval {
    per_record: PerRecordResult,
    cm: ConfusionMatrix4,
    failures: Vec<(BeatClassification, RhythmClass)>,
}

fn evaluate_record(
    record_id: &str,
    beats: &[BeatRecord],
    warmup: usize,
    win: &RateRegimeWindow,
) -> RecordEval {
    // Pre-compute the per-beat ground-truth labels via the
    // sliding-window helper. We use the RR intervals encoded in the
    // CSV (annotation-derived) — never the detector's own BPM.
    let rr_ms: Vec<u32> = beats
        .iter()
        .map(|b| b.rr_interval_ms.max(1.0) as u32)
        .collect();
    let truth_per_beat: Vec<Option<RhythmClass>> = (0..beats.len())
        .map(|i| rate_regime_truth(&rr_ms, i, win))
        .collect();

    // Build the frequency stream and run the detector.
    let stream = cricket_brain_cardiac::ecg_signal::beats_to_frequency_stream(beats);
    let mut det = CardiacDetector::new();
    let preds = det.classify_stream(beats);

    // For each emission, look up the closest beat by step → timestamp.
    // Each beat occupies [previous_qrs_end, current_qrs_end]. With the
    // synthetic stream this maps emission step to beat index reliably.
    let mut cm = ConfusionMatrix4::new();
    let mut n_correct = 0;
    let mut n_truth = 0;
    let mut failures = Vec::new();

    // Pre-compute cumulative step boundaries — beat i ends at
    // sum_{j<=i} rr_j. We measure in steps; rr in ms = 1 step.
    let mut beat_end_step: Vec<usize> = Vec::with_capacity(beats.len());
    let mut acc: usize = 0;
    for b in beats {
        acc = acc.saturating_add(b.rr_interval_ms as usize);
        beat_end_step.push(acc);
    }

    let mut last_truth_seen: Option<RhythmClass> = None;
    let mut warmup_skipped: usize = 0;
    for p in &preds {
        // Find smallest beat index whose end_step >= p.step.
        let beat_idx = beat_end_step.partition_point(|&end| end < p.step);
        if beat_idx >= beats.len() {
            continue;
        }
        let truth = match truth_per_beat[beat_idx] {
            Some(t) => t,
            None => continue, // window warmup at start of record
        };
        if Some(truth) != last_truth_seen {
            last_truth_seen = Some(truth);
            warmup_skipped = 0;
        }
        warmup_skipped += 1;
        if warmup_skipped <= warmup {
            continue;
        }
        cm.record(truth, p.rhythm);
        n_truth += 1;
        if truth == p.rhythm {
            n_correct += 1;
        } else {
            failures.push((p.clone(), truth));
        }
    }

    let aami = aami_counts_for(beats);
    let per_record = PerRecordResult {
        record_id: record_id.to_string(),
        n_beats: beats.len(),
        n_emissions: preds.len(),
        n_ground_truth: n_truth,
        n_correct,
        aami_counts: aami,
    };

    let _ = stream; // not used directly; classify_stream rebuilds it
    RecordEval {
        per_record,
        cm,
        failures,
    }
}

fn print_per_record_table(records: &[RecordEval]) {
    println!("\n  Per-record results:");
    println!(
        "  {:<14} {:>8} {:>10} {:>10} {:>10}  {:>5} {:>5} {:>5} {:>5} {:>5}",
        "record_id", "n_beats", "emissions", "truth", "accuracy", "N", "S", "V", "F", "Q"
    );
    for r in records {
        let p = &r.per_record;
        let agg = AggregateMetrics::from_cm(&r.cm);
        println!(
            "  {:<14} {:>8} {:>10} {:>10} {:>10.4}  {:>5} {:>5} {:>5} {:>5} {:>5}",
            p.record_id,
            p.n_beats,
            p.n_emissions,
            p.n_ground_truth,
            agg.accuracy,
            p.aami_counts[0],
            p.aami_counts[1],
            p.aami_counts[2],
            p.aami_counts[3],
            p.aami_counts[4],
        );
    }
}

fn pooled_confusion(records: &[RecordEval]) -> ConfusionMatrix4 {
    let mut cm = ConfusionMatrix4::new();
    for r in records {
        for t in 0..NUM_CLASSES {
            for p in 0..NUM_CLASSES {
                cm.m[t][p] += r.cm.m[t][p];
            }
        }
    }
    cm
}

fn write_skeleton_only_status(meta: &RunMetadata, records: &[(String, Vec<BeatRecord>)]) {
    let mut report = JsonReport::new(meta.clone());
    let n_total: usize = records.iter().map(|(_, b)| b.len()).sum();
    report.add_block(
        "status",
        json_object(&[
            ("v", "\"skeleton_only\"".to_string()),
            (
                "reason",
                "\"all loaded record_ids are synthetic\"".to_string(),
            ),
            ("n_records_seen", records.len().to_string()),
            ("n_beats_seen", n_total.to_string()),
        ]),
    );
    let ids: Vec<String> = records.iter().map(|(id, _)| format!("\"{id}\"")).collect();
    report.add_block("record_ids", json_array(&ids));
    report.add_block(
        "next_step",
        json_quote(
            "Run python python/download_mitbih.py and python python/preprocess.py to populate \
             data/processed/{train,test}/, then re-run with --records-dir <those>.",
        ),
    );
    let path = PathBuf::from(RESULT_DIR).join("cardiac_mitbih_skeleton_only.json");
    report.write_to(&path).expect("write skeleton json");
    println!("  wrote {}", path.display());
}

fn write_real_results(
    meta: &RunMetadata,
    records: &[RecordEval],
    pooled_cm: &ConfusionMatrix4,
    pooled: &PooledResult,
) {
    // ---------------- JSON summary ----------------
    let mut report = JsonReport::new(meta.clone());
    let agg = AggregateMetrics::from_cm(pooled_cm);
    report.add_block("status", json_object(&[("v", "\"real_data\"".to_string())]));
    report.add_block(
        "pooled",
        json_object(&[
            ("n_records", records.len().to_string()),
            ("total_beats", pooled.total_beats().to_string()),
            ("micro_accuracy", json_f64(pooled.micro_accuracy())),
            (
                "macro_accuracy_over_records",
                json_f64(pooled.macro_accuracy()),
            ),
            ("macro_f1", json_f64(agg.macro_f1)),
            ("weighted_f1", json_f64(agg.weighted_f1)),
            ("balanced_accuracy", json_f64(agg.balanced_accuracy)),
        ]),
    );

    // Per-record array
    let per_record_blocks: Vec<String> = records
        .iter()
        .map(|r| {
            let p = &r.per_record;
            let agg = AggregateMetrics::from_cm(&r.cm);
            json_object(&[
                ("record_id", format!("\"{}\"", p.record_id)),
                ("n_beats", p.n_beats.to_string()),
                ("n_emissions", p.n_emissions.to_string()),
                ("n_ground_truth", p.n_ground_truth.to_string()),
                ("n_correct", p.n_correct.to_string()),
                ("accuracy", json_f64(p.accuracy())),
                ("macro_f1", json_f64(agg.macro_f1)),
                (
                    "aami_counts",
                    format!(
                        "{{\"N\": {}, \"S\": {}, \"V\": {}, \"F\": {}, \"Q\": {}}}",
                        p.aami_counts[0],
                        p.aami_counts[1],
                        p.aami_counts[2],
                        p.aami_counts[3],
                        p.aami_counts[4],
                    ),
                ),
            ])
        })
        .collect();
    report.add_block("per_record", json_array(&per_record_blocks));

    let json_path = PathBuf::from(RESULT_DIR).join("cardiac_mitbih_summary.json");
    report.write_to(&json_path).expect("write json");
    println!("  wrote {}", json_path.display());

    // ---------------- per-record CSV ----------------
    let mut csv = String::from(
        "record_id,n_beats,n_emissions,n_ground_truth,n_correct,accuracy,macro_f1,recall_Normal,recall_Tachy,recall_Brady,recall_Irregular,aami_N,aami_S,aami_V,aami_F,aami_Q\n",
    );
    for r in records {
        let p = &r.per_record;
        let agg = AggregateMetrics::from_cm(&r.cm);
        csv.push_str(&format!(
            "{},{},{},{},{},{:.6},{:.6}",
            p.record_id,
            p.n_beats,
            p.n_emissions,
            p.n_ground_truth,
            p.n_correct,
            p.accuracy(),
            agg.macro_f1
        ));
        for i in 0..NUM_CLASSES {
            csv.push(',');
            csv.push_str(&format!("{:.6}", r.cm.recall(class_from_index(i))));
        }
        for c in p.aami_counts {
            csv.push(',');
            csv.push_str(&c.to_string());
        }
        csv.push('\n');
    }
    let csv_path = PathBuf::from(RESULT_DIR).join("cardiac_mitbih_per_record.csv");
    write_csv_with_header(&csv_path, meta, &csv).expect("write csv");
    println!("  wrote {}", csv_path.display());

    // ---------------- failure cases ----------------
    let mut md = String::from("# UC01 MIT-BIH — Failure Cases (real-data run)\n\n");
    md.push_str(&format!(
        "Generated: {} | dataset: `{}`\n\n",
        meta.generated_at, meta.dataset_name
    ));
    md.push_str(
        "Each row is one emission whose **prediction differed from the rate-regime ground truth** \
         derived from the annotation RR intervals (sliding 5-beat window, see `mitbih::rate_regime_truth`). \
         AAMI symbol distributions are reported per record for traceability only — the detector \
         is rate-based and does not classify AAMI morphology.\n\n",
    );
    md.push_str("| record | step | truth | predicted | bpm | confidence |\n");
    md.push_str("|--------|-----:|:------|:----------|----:|-----------:|\n");
    let mut shown = 0;
    for r in records {
        for (p, truth) in &r.failures {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {:.0} | {:.2} |\n",
                r.per_record.record_id,
                p.step,
                class_label(*truth),
                class_label(p.rhythm),
                p.bpm,
                p.confidence,
            ));
            shown += 1;
            if shown >= 200 {
                break;
            }
        }
        if shown >= 200 {
            break;
        }
    }
    if shown == 0 {
        md.push_str("| — | — | — | — | — | — |\n\n_All emissions correct after warmup._\n");
    }
    let md_path = PathBuf::from(RESULT_DIR).join("cardiac_mitbih_failure_cases.md");
    if let Some(parent) = md_path.parent() {
        std::fs::create_dir_all(parent).expect("create results dir");
    }
    std::fs::write(&md_path, md).expect("write md");
    println!("  wrote {}", md_path.display());
}

fn main() {
    let args = Args::parse();
    println!("== Cardiac MIT-BIH (v0.3 patient-aware loader) ==");

    let records = load_records(&args);
    if records.is_empty() {
        eprintln!("ERROR: no beats loaded.");
        eprintln!(
            "Pass --csv <path> or --records-dir <dir>, or run \
             `python python/preprocess.py --synthetic` to regenerate the bundled \
             synthetic sample."
        );
        std::process::exit(1);
    }

    println!(
        "  loaded {} record(s), {} total beats",
        records.len(),
        records.iter().map(|(_, b)| b.len()).sum::<usize>()
    );
    for (id, beats) in &records {
        let aami = aami_counts_for(beats);
        println!(
            "    record_id={:<14} beats={:>6}  AAMI N/S/V/F/Q = {}/{}/{}/{}/{}",
            id,
            beats.len(),
            aami[0],
            aami[1],
            aami[2],
            aami[3],
            aami[4]
        );
    }

    let synthetic_only = all_synthetic(&records);
    if synthetic_only {
        println!(
            "\n  STATUS: every loaded record_id is synthetic. The bench refuses to \n\
             produce 'validated' result files — no MIT-BIH numbers will be published.\n\
             To run on real MIT-BIH data, see use_cases/01_cardiac_arrhythmia/data/SOURCES.md."
        );
    }

    let win = RateRegimeWindow::default();
    let evaluations: Vec<RecordEval> = records
        .iter()
        .map(|(id, beats)| evaluate_record(id, beats, args.warmup, &win))
        .collect();

    print_per_record_table(&evaluations);

    let pooled_cm = pooled_confusion(&evaluations);
    let pooled = PooledResult {
        records: evaluations.iter().map(|r| r.per_record.clone()).collect(),
    };
    pooled_cm.print(&format!(
        "\nPooled confusion matrix across {} record(s):",
        evaluations.len()
    ));
    println!(
        "  micro_accuracy = {:.4}   macro_accuracy_over_records = {:.4}",
        pooled.micro_accuracy(),
        pooled.macro_accuracy(),
    );

    if !args.write {
        println!("\n  (--write not passed → result files not regenerated)");
        return;
    }

    let cmd = current_command_line();
    let dataset_name = if synthetic_only {
        "uc01_mitbih_synth_sample".to_string()
    } else {
        format!("uc01_mitbih_{}_records", evaluations.len())
    };
    let dataset_type = if synthetic_only {
        "synthetic"
    } else {
        "mitbih_csv"
    };
    let mut meta = RunMetadata::new(
        &cmd,
        dataset_type,
        &dataset_name,
        0,
        &["Normal", "Tachy", "Brady", "Irregular"],
        1,
        1000,
    );

    // Real-data limitations differ from the synthetic ones. Override the
    // default limitations field so the published JSON tells the truth.
    if !synthetic_only {
        meta.limitations = vec![
            "Real MIT-BIH Arrhythmia Database records (PhysioNet, ODC-By v1.0).".into(),
            "Rate-regime triage only — Normal / Tachy / Brady / Irregular. \
             Does NOT classify AAMI N/S/V/F/Q morphology, AF, VT, AVB, BBB, ST-elevation."
                .into(),
            "Ground truth derived from annotation RR intervals via a 5-beat sliding window \
             (mitbih::rate_regime_truth); not a clinician rhythm label."
                .into(),
            "No inter-patient train/test split assertion in this binary — caller chooses \
             which directory to evaluate."
                .into(),
            "Not a medical device. Not validated for clinical use. Research / embedded \
             pre-screening prototype only."
                .into(),
        ];
    }

    if synthetic_only {
        write_skeleton_only_status(&meta, &records);
    } else {
        write_real_results(&meta, &evaluations, &pooled_cm, &pooled);
    }
}
