// SPDX-License-Identifier: AGPL-3.0-only
//! Latency & throughput benchmark for the marine acoustic detector.
//! Date: 2026-04-24

use cricket_brain_marine::acoustic_signal;
use cricket_brain_marine::detector::MarineDetector;
use std::time::Instant;

const RUNS: usize = 100;

fn measure(name: &str, signal_fn: fn(usize) -> Vec<f32>, n_steps: usize) {
    let mut det = MarineDetector::new();
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

    println!("  {name:35} {mean_lat:>8.0} ms  {mean_us:>10.3} us/step");
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Marine Latency Benchmark                                  ║");
    println!("║  Date: 2026-04-24 | {RUNS} runs per condition               ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("  {:<35} {:>8}    {:>10}", "Condition", "Latency", "Speed");
    println!("  {:─>35} {:─>8}    {:─>10}", "", "", "");

    measure("Ambient", acoustic_signal::ambient_noise, 500);
    measure("Fin Whale (20 Hz)", acoustic_signal::fin_whale_call, 500);
    measure("Blue Whale (80 Hz)", acoustic_signal::blue_whale_call, 500);
    measure("Ship Noise (140 Hz)", acoustic_signal::ship_passage, 500);
    measure("Humpback (200 Hz)", acoustic_signal::humpback_song, 500);

    let det = MarineDetector::new();
    println!(
        "\n  RAM: {} bytes ({} neurons)",
        det.memory_usage_bytes(),
        det.total_neurons()
    );
}
