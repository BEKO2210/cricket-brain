// SPDX-License-Identifier: AGPL-3.0-only
//! Power-grid harmonic & stability triage demo.
//!
//! Modes:
//!   cargo run                                            — synthetic demo
//!   cargo run -- --csv data/processed/sample_grid.csv    — classify CSV
//!   cargo run -- --factory                               — factory-startup transient
//!   cargo run -- --brownout                              — rolling brownout

use cricket_brain_grid::detector::{ConfusionMatrix, GridDetector};
use cricket_brain_grid::grid_signal;
use std::time::Instant;

const STEPS_PER_WINDOW: usize = 25;

fn run_scenario(label: &str, signal: &[f32], det: &mut GridDetector) {
    det.reset();
    let t0 = Instant::now();
    let mut classifications = Vec::new();

    for &freq in signal {
        if let Some(event) = det.step(freq) {
            classifications.push((event, det.confidence(), det.steps_processed()));
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

    for (i, (event, conf, step)) in classifications.iter().enumerate() {
        println!(
            "  Window {}: {} | Confidence={:.2} | Step={}",
            i + 1,
            event,
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

    let windows = grid_signal::from_csv(csv_path);
    if windows.is_empty() {
        eprintln!("ERROR: No windows in {csv_path}");
        std::process::exit(1);
    }
    println!("Loaded: {} grid windows\n", windows.len());

    let mut det = GridDetector::new();
    let t0 = Instant::now();
    let results = det.classify_stream(&windows, STEPS_PER_WINDOW);
    let elapsed = t0.elapsed();

    for (i, r) in results.iter().enumerate() {
        println!(
            "  [{:>4}] {} | Conf={:.2} | Step={}",
            i + 1,
            r.event,
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

fn run_factory_demo() {
    println!("=== Factory Startup Transient Demo ===");
    println!("=== Nominal grid → VFD load adds 3rd-harmonic distortion → recovery ===\n");
    let sig = grid_signal::factory_startup(1500, 500);
    let mut det = GridDetector::new();
    println!("  Step    | Classification        | Conf");
    println!("  {:-<48}", "");
    for &f in &sig {
        if let Some(ev) = det.step(f) {
            println!(
                "  {:>7} | {:<25} | {:.2}",
                det.steps_processed(),
                format!("{}", ev),
                det.confidence()
            );
        }
    }
    println!(
        "\n  RAM: {} bytes ({} neurons)\n",
        det.memory_usage_bytes(),
        det.total_neurons()
    );
}

fn run_brownout_demo() {
    println!("=== Rolling Brownout Demo (4 dips, 80 steps each) ===\n");
    let sig = grid_signal::rolling_brownout(2000, 4, 80);
    let mut det = GridDetector::new();
    println!("  Step    | Classification        | Conf");
    println!("  {:-<48}", "");
    for &f in &sig {
        if let Some(ev) = det.step(f) {
            println!(
                "  {:>7} | {:<25} | {:.2}",
                det.steps_processed(),
                format!("{}", ev),
                det.confidence()
            );
        }
    }
    println!();
}

fn run_synthetic_demo() {
    println!("=== CricketBrain Power-Grid Triage ===");
    println!("=== EPFL-style PMU stream — 4-channel ResonatorBank ===\n");

    let mut det = GridDetector::new();

    run_scenario("Outage", &grid_signal::outage(500), &mut det);
    run_scenario("Nominal Grid (50 Hz)", &grid_signal::nominal_grid(500), &mut det);
    run_scenario(
        "2nd Harmonic (100 Hz — DC offset / saturation)",
        &grid_signal::second_harmonic_dominant(500),
        &mut det,
    );
    run_scenario(
        "3rd Harmonic (150 Hz — non-linear loads)",
        &grid_signal::third_harmonic_dominant(500),
        &mut det,
    );
    run_scenario(
        "4th Harmonic (200 Hz — switching artefacts)",
        &grid_signal::fourth_harmonic_dominant(500),
        &mut det,
    );

    println!("--- Mixed: Nominal → Outage → 3rd-harmonic disturbance → Recovery ---");
    det.reset();
    let mut signal = grid_signal::nominal_grid(200);
    signal.extend(grid_signal::outage(150));
    signal.extend(grid_signal::third_harmonic_dominant(250));
    signal.extend(grid_signal::nominal_grid(200));

    let mut n = 0;
    for &freq in &signal {
        if let Some(event) = det.step(freq) {
            n += 1;
            println!(
                "  Window {}: {} | Confidence={:.2}",
                n,
                event,
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
    } else if args.iter().any(|a| a == "--factory") {
        run_factory_demo();
    } else if args.iter().any(|a| a == "--brownout") {
        run_brownout_demo();
    } else {
        run_synthetic_demo();
    }
}
