// SPDX-License-Identifier: AGPL-3.0-only
//! Marine acoustic monitoring demo.
//!
//! Modes:
//!   cargo run                                           — synthetic demo
//!   cargo run -- --csv data/processed/sample_marine.csv — classify CSV
//!   cargo run -- --ship-transit                         — ship sailing past

use cricket_brain_marine::acoustic_signal;
use cricket_brain_marine::detector::{ConfusionMatrix, MarineDetector};
use std::time::Instant;

const STEPS_PER_WINDOW: usize = 25;

fn run_scenario(label: &str, signal: &[f32], det: &mut MarineDetector) {
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

    let windows = acoustic_signal::from_csv(csv_path);
    if windows.is_empty() {
        eprintln!("ERROR: No windows in {csv_path}");
        std::process::exit(1);
    }
    println!("Loaded: {} hydrophone windows\n", windows.len());

    let mut det = MarineDetector::new();
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

fn run_ship_transit_demo() {
    println!("=== Ship Transit Demo — cargo vessel sailing past the hydrophone ===\n");
    let sig = acoustic_signal::ship_transit(2000);
    let mut det = MarineDetector::new();

    println!("  Step    | Classification        | Conf");
    println!("  {:-<40}", "");
    for &f in &sig {
        if let Some(ev) = det.step(f) {
            println!(
                "  {:>7} | {:<22} | {:.2}",
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

fn run_synthetic_demo() {
    println!("=== CricketBrain Marine Acoustic Monitoring ===");
    println!("=== MBARI MARS hydrophone — 4-channel ResonatorBank ===\n");

    let mut det = MarineDetector::new();

    run_scenario(
        "Ambient Ocean",
        &acoustic_signal::ambient_noise(500),
        &mut det,
    );
    run_scenario(
        "Fin Whale 20-Hz Pulse",
        &acoustic_signal::fin_whale_call(500),
        &mut det,
    );
    run_scenario(
        "Blue Whale A-Call (80 Hz)",
        &acoustic_signal::blue_whale_call(500),
        &mut det,
    );
    run_scenario(
        "Ship Passage (140 Hz cavitation)",
        &acoustic_signal::ship_passage(500),
        &mut det,
    );
    run_scenario(
        "Humpback Song (200 Hz)",
        &acoustic_signal::humpback_song(500),
        &mut det,
    );

    println!("--- Mixed: Ambient → Ship Approach → Whale Vocalising ---");
    det.reset();
    let mut signal = acoustic_signal::ambient_noise(200);
    signal.extend(acoustic_signal::ship_passage(300));
    signal.extend(acoustic_signal::fin_whale_call(300));

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
    } else if args.iter().any(|a| a == "--ship-transit") {
        run_ship_transit_demo();
    } else {
        run_synthetic_demo();
    }
}
