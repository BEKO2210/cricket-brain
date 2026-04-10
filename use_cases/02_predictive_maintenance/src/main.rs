// SPDX-License-Identifier: AGPL-3.0-only
//! Bearing fault detection demo.
//!
//! Modes:
//!   cargo run                                       — synthetic demo
//!   cargo run -- --csv data/processed/sample.csv    — classify CSV

use cricket_brain_bearings::detector::{BearingDetector, ConfusionMatrix};
use cricket_brain_bearings::vibration_signal;
use std::time::Instant;

const STEPS_PER_WINDOW: usize = 25;

fn run_scenario(label: &str, signal: &[f32], det: &mut BearingDetector) {
    det.reset();
    let t0 = Instant::now();
    let mut classifications = Vec::new();

    for &freq in signal {
        if let Some(fault) = det.step(freq) {
            classifications.push((fault, det.confidence(), det.steps_processed()));
        }
    }

    let elapsed = t0.elapsed();
    let us_per_step = elapsed.as_secs_f64() * 1_000_000.0 / signal.len() as f64;

    println!("--- {label} ({} steps) ---", signal.len());
    println!(
        "  Latency: {:.3} us/step | RAM: {} bytes ({} neurons)",
        us_per_step,
        det.memory_usage_bytes(),
        det.total_neurons()
    );

    if classifications.is_empty() {
        println!("  No classifications\n");
        return;
    }

    for (i, (fault, conf, step)) in classifications.iter().enumerate() {
        println!(
            "  Window {}: {} | Confidence={:.2} | Step={}",
            i + 1,
            fault,
            conf,
            step
        );
    }

    let last = &classifications[classifications.len() - 1];
    println!("  Final: {} (Conf={:.2})\n", last.0, last.1);
}

fn run_csv_mode(csv_path: &str) {
    println!("=== CSV Classification Mode ===\n");
    println!("Input: {csv_path}");

    let windows = vibration_signal::from_csv(csv_path);
    if windows.is_empty() {
        eprintln!("ERROR: No windows in {csv_path}");
        std::process::exit(1);
    }
    println!("Loaded: {} frequency windows\n", windows.len());

    let mut det = BearingDetector::new();
    let t0 = Instant::now();
    let results = det.classify_stream(&windows, STEPS_PER_WINDOW);
    let elapsed = t0.elapsed();

    for (i, r) in results.iter().enumerate() {
        println!(
            "  [{:>4}] {} | Conf={:.2} | Step={}",
            i + 1,
            r.fault,
            r.confidence,
            r.step
        );
    }

    println!(
        "\n  Total: {} classifications from {} windows in {:.3} ms",
        results.len(),
        windows.len(),
        elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "  RAM: {} bytes ({} neurons)\n",
        det.memory_usage_bytes(),
        det.total_neurons()
    );

    let cm = ConfusionMatrix::from_predictions(&results, &windows, STEPS_PER_WINDOW);
    cm.print();
}

fn run_synthetic_demo() {
    println!("=== CricketBrain Bearing Fault Detection ===");
    println!("=== CWRU SKF 6205-2RS — 4-channel ResonatorBank ===\n");

    let mut det = BearingDetector::new();

    run_scenario(
        "Normal (no fault)",
        &vibration_signal::normal_vibration(500),
        &mut det,
    );
    run_scenario(
        "Outer Race Fault (BPFO 107 Hz)",
        &vibration_signal::outer_race_fault(500),
        &mut det,
    );
    run_scenario(
        "Inner Race Fault (BPFI 162 Hz)",
        &vibration_signal::inner_race_fault(500),
        &mut det,
    );
    run_scenario(
        "Ball Defect (BSF 69 Hz)",
        &vibration_signal::ball_fault(500),
        &mut det,
    );

    // Mixed
    println!("--- Mixed: Normal → Outer Race Fault Onset ---");
    det.reset();
    let mut signal = vibration_signal::normal_vibration(300);
    signal.extend(vibration_signal::outer_race_fault(300));

    let mut n = 0;
    for &freq in &signal {
        if let Some(fault) = det.step(freq) {
            n += 1;
            println!(
                "  Window {}: {} | Confidence={:.2}",
                n,
                fault,
                det.confidence()
            );
        }
    }
    println!();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if let Some(pos) = args.iter().position(|a| a == "--csv") {
        if let Some(path) = args.get(pos + 1) {
            run_csv_mode(path);
        } else {
            eprintln!("ERROR: --csv requires a file path");
            std::process::exit(1);
        }
    } else {
        run_synthetic_demo();
    }
}
