// SPDX-License-Identifier: AGPL-3.0-only
//! Latency & throughput benchmark for the grid event detector.
//! Date: 2026-04-24

use cricket_brain_grid::detector::GridDetector;
use cricket_brain_grid::grid_signal;
use std::time::Instant;

const RUNS: usize = 100;

fn measure(name: &str, signal_fn: fn(usize) -> Vec<f32>, n_steps: usize) {
    let mut det = GridDetector::new();
    let mut latencies = Vec::with_capacity(RUNS);
    let mut speeds = Vec::with_capacity(RUNS);

    for _ in 0..RUNS {
        det.reset();
        let sig = signal_fn(n_steps);
        let t0 = Instant::now();
        let mut first = None;
        for (i, &f) in sig.iter().enumerate() {
            if det.step(f).is_some() && first.is_none() {
                first = Some(i);
            }
        }
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0 / n_steps as f64;
        speeds.push(us);
        if let Some(s) = first {
            latencies.push(s as f32);
        }
    }

    let mean_lat = latencies.iter().sum::<f32>() / latencies.len().max(1) as f32;
    let mean_us = speeds.iter().sum::<f64>() / speeds.len().max(1) as f64;
    println!("  {name:38} {mean_lat:>8.0} ms  {mean_us:>10.3} us/step");
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Grid Latency Benchmark                                    ║");
    println!("║  Date: 2026-04-24 | {RUNS} runs per condition               ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("  {:<38} {:>8}    {:>10}", "Condition", "Latency", "Speed");
    println!("  {:─>38} {:─>8}    {:─>10}", "", "", "");

    measure("Outage", grid_signal::outage, 500);
    measure("Nominal (50 Hz)", grid_signal::nominal_grid, 500);
    measure("2nd Harmonic (100 Hz)", grid_signal::second_harmonic_dominant, 500);
    measure("3rd Harmonic (150 Hz)", grid_signal::third_harmonic_dominant, 500);
    measure("4th Harmonic (200 Hz)", grid_signal::fourth_harmonic_dominant, 500);

    let det = GridDetector::new();
    println!(
        "\n  RAM: {} bytes ({} neurons)",
        det.memory_usage_bytes(),
        det.total_neurons()
    );
}
