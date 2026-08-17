// SPDX-License-Identifier: AGPL-3.0-only
//! Stress-test sweeps for `CardiacDetector`.
//!
//! For each stress dimension we sweep a parameter and measure how the
//! detector's macro-F1 / accuracy / per-class recall degrade. Outputs
//! are CSV files with metadata headers, suitable for plotting.
//!
//! Dimensions:
//!
//! 1. `noise` — random in-band noise spike probability
//! 2. `baseline_wander` — slow sinusoidal frequency drift amplitude
//! 3. `amp_jitter` — per-beat QRS frequency jitter
//! 4. `hrv` — RR-interval jitter
//! 5. `morph_jitter` — per-cycle morphology jitter (P/QRS/T freq+dur)
//! 6. `missing_qrs` — probability of dropping a QRS burst entirely
//! 7. `motion_burst` — probability of broadband motion-artifact bursts
//!
//! Reproduce:
//! ```
//! cargo run --release --example cardiac_stress_sweep -- --seed 42
//! cargo run --release --example cardiac_stress_sweep -- --seed 42 --write
//! ```

use std::path::PathBuf;

use cricket_brain_cardiac::detector::CardiacDetector;
use cricket_brain_cardiac::evaluate::{confusion_matrix, drop_warmup, run_and_score};
use cricket_brain_cardiac::metrics::{
    class_from_index, class_label, AggregateMetrics, ConfusionMatrix4, NUM_CLASSES,
};
use cricket_brain_cardiac::report::{current_command_line, write_csv_with_header, RunMetadata};
use cricket_brain_cardiac::synthetic::{generate, SyntheticConfig};

const RESULT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/results");

struct Args {
    seed: u64,
    beats_per_class: u32,
    warmup: usize,
    preprocessor: bool,
    write: bool,
}

impl Args {
    fn parse() -> Self {
        let mut a = Args {
            seed: 42,
            beats_per_class: 20,
            warmup: 2,
            preprocessor: false,
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
                "--preprocessor" => a.preprocessor = true,
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

fn evaluate(cfg: &SyntheticConfig, args: &Args) -> (AggregateMetrics, ConfusionMatrix4) {
    let rec = generate(cfg);
    let mut det = if args.preprocessor {
        CardiacDetector::with_preprocessor(true)
    } else {
        CardiacDetector::new()
    };
    let scored = run_and_score(&mut det, &rec);
    let scored = drop_warmup(&scored, args.warmup);
    let cm = confusion_matrix(&scored);
    (AggregateMetrics::from_cm(&cm), cm)
}

fn csv_header() -> String {
    let mut h = String::from(
        "dimension,parameter,value,total,correct,accuracy,macro_f1,weighted_f1,balanced_accuracy",
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

fn csv_row(
    dimension: &str,
    parameter: &str,
    value: f64,
    agg: &AggregateMetrics,
    cm: &ConfusionMatrix4,
) -> String {
    let mut out = format!(
        "{},{},{},{},{},{:.6},{:.6},{:.6},{:.6}",
        dimension,
        parameter,
        value,
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

fn print_row(
    dimension: &str,
    parameter: &str,
    value: f64,
    agg: &AggregateMetrics,
    cm: &ConfusionMatrix4,
) {
    print!(
        "  {:<18} {:<20} value={:>8.4}  acc={:.3}  macroF1={:.3}",
        dimension, parameter, value, agg.accuracy, agg.macro_f1
    );
    for i in 0..NUM_CLASSES {
        let c = class_from_index(i);
        let r = cm.recall(c);
        if cm.support(c) > 0 {
            print!("  R[{}]={:.2}", class_label(c), r);
        }
    }
    println!();
}

fn run_dimension<F: Fn(&mut SyntheticConfig, f64)>(
    args: &Args,
    dimension: &str,
    parameter: &str,
    sweep: &[f64],
    apply: F,
    out: &mut String,
) {
    println!("\n=== Sweep: {dimension} ===");
    for &v in sweep {
        let mut cfg = SyntheticConfig::default()
            .with_seed(args.seed)
            .with_beats_per_class(args.beats_per_class)
            .with_irregular(true);
        apply(&mut cfg, v);
        let (agg, cm) = evaluate(&cfg, args);
        print_row(dimension, parameter, v, &agg, &cm);
        out.push_str(&csv_row(dimension, parameter, v, &agg, &cm));
    }
}

fn main() {
    let args = Args::parse();
    println!("== Cardiac Stress Sweep ==");
    println!(
        "  seed={}  beats/class={}  warmup={}  preprocessor={}  write={}",
        args.seed, args.beats_per_class, args.warmup, args.preprocessor, args.write,
    );

    let mut csv = csv_header();

    // Baseline (clean signal)
    {
        let cfg = SyntheticConfig::default()
            .with_seed(args.seed)
            .with_beats_per_class(args.beats_per_class)
            .with_irregular(true);
        let (agg, cm) = evaluate(&cfg, &args);
        print_row("baseline", "clean", 0.0, &agg, &cm);
        csv.push_str(&csv_row("baseline", "clean", 0.0, &agg, &cm));
    }

    run_dimension(
        &args,
        "noise",
        "noise_prob",
        &[0.0, 0.005, 0.01, 0.02, 0.05, 0.10, 0.20, 0.30],
        |cfg, v| cfg.noise_prob = v as f32,
        &mut csv,
    );

    run_dimension(
        &args,
        "baseline_wander",
        "wander_hz",
        &[0.0, 50.0, 100.0, 200.0, 400.0, 800.0],
        |cfg, v| cfg.baseline_wander_hz = v as f32,
        &mut csv,
    );

    run_dimension(
        &args,
        "amp_jitter",
        "qrs_freq_jitter",
        &[0.0, 0.02, 0.05, 0.10, 0.15, 0.20],
        |cfg, v| cfg.amp_jitter = v as f32,
        &mut csv,
    );

    run_dimension(
        &args,
        "hrv",
        "rr_jitter",
        &[0.0, 0.02, 0.05, 0.10, 0.20, 0.35],
        |cfg, v| cfg.hrv = v as f32,
        &mut csv,
    );

    run_dimension(
        &args,
        "morph_jitter",
        "wave_jitter",
        &[0.0, 0.05, 0.10, 0.20, 0.35],
        |cfg, v| cfg.morph_jitter = v as f32,
        &mut csv,
    );

    run_dimension(
        &args,
        "missing_qrs",
        "drop_prob",
        &[0.0, 0.05, 0.10, 0.20, 0.40],
        |cfg, v| cfg.missing_qrs_prob = v as f32,
        &mut csv,
    );

    run_dimension(
        &args,
        "motion_burst",
        "burst_prob",
        &[0.0, 0.0001, 0.0005, 0.001, 0.002, 0.005],
        |cfg, v| cfg.motion_burst_prob = v as f32,
        &mut csv,
    );

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
    let path = PathBuf::from(RESULT_DIR).join("cardiac_stress_sweep.csv");
    write_csv_with_header(&path, &meta, &csv).expect("write csv");
    println!("\n  wrote {}", path.display());

    // Per-dimension splits — dataset reviewers usually look at one
    // dimension at a time.
    for dim in [
        "noise",
        "baseline_wander",
        "amp_jitter",
        "hrv",
        "morph_jitter",
        "missing_qrs",
        "motion_burst",
    ] {
        let mut split = csv_header();
        for line in csv.lines().skip(1) {
            if line.starts_with(dim) || line.starts_with("baseline,") {
                split.push_str(line);
                split.push('\n');
            }
        }
        let split_path = PathBuf::from(RESULT_DIR).join(format!("cardiac_stress_{dim}.csv"));
        write_csv_with_header(&split_path, &meta, &split).expect("write split");
        println!("  wrote {}", split_path.display());
    }
}
