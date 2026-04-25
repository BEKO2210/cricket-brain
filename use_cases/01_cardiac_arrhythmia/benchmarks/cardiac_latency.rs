// SPDX-License-Identifier: AGPL-3.0-only
//! Latency benchmark for cardiac rhythm detection.
//!
//! Measures:
//! - First-classification latency per rhythm type (ms from signal start)
//! - Processing speed (µs/step)
//! - Throughput (steps/sec)
//!
//! Date: 2026-04-10

use cricket_brain_cardiac::detector::CardiacDetector;
use cricket_brain_cardiac::ecg_signal;
use std::time::Instant;

const RUNS: usize = 100;

struct LatencyStats {
    name: &'static str,
    latencies_ms: Vec<f32>,
    us_per_step: Vec<f64>,
}

impl LatencyStats {
    fn mean_latency(&self) -> f32 {
        self.latencies_ms.iter().sum::<f32>() / self.latencies_ms.len().max(1) as f32
    }
    fn sd_latency(&self) -> f32 {
        let m = self.mean_latency();
        let var = self
            .latencies_ms
            .iter()
            .map(|&l| (l - m) * (l - m))
            .sum::<f32>()
            / self.latencies_ms.len().max(1) as f32;
        var.sqrt()
    }
    fn min_latency(&self) -> f32 {
        self.latencies_ms.iter().cloned().fold(f32::MAX, f32::min)
    }
    fn max_latency(&self) -> f32 {
        self.latencies_ms.iter().cloned().fold(0.0f32, f32::max)
    }
    fn mean_us_per_step(&self) -> f64 {
        self.us_per_step.iter().sum::<f64>() / self.us_per_step.len().max(1) as f64
    }
}

fn measure_latency(
    name: &'static str,
    cycle: &ecg_signal::EcgCycle,
    n_cycles: usize,
) -> LatencyStats {
    let mut latencies = Vec::with_capacity(RUNS);
    let mut speeds = Vec::with_capacity(RUNS);
    let mut det = CardiacDetector::new();

    for _ in 0..RUNS {
        det.reset();
        let stream = cycle.to_frequency_stream(n_cycles);
        let t0 = Instant::now();
        let mut first_step = None;

        for (i, &freq) in stream.iter().enumerate() {
            if det.step(freq).is_some() && first_step.is_none() {
                first_step = Some(i);
            }
        }

        let elapsed = t0.elapsed();
        let us = elapsed.as_secs_f64() * 1_000_000.0 / stream.len() as f64;
        speeds.push(us);

        if let Some(step) = first_step {
            latencies.push(step as f32); // 1 step = 1 ms
        }
    }

    LatencyStats {
        name,
        latencies_ms: latencies,
        us_per_step: speeds,
    }
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Cardiac Latency Benchmark                                 ║");
    println!("║  Date: 2026-04-10 | Runs: {RUNS} per condition              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let results = vec![
        measure_latency("Normal Sinus (73 BPM)", &ecg_signal::normal_sinus(), 8),
        measure_latency("Tachycardia (150 BPM)", &ecg_signal::tachycardia(), 8),
        measure_latency("Bradycardia (40 BPM)", &ecg_signal::bradycardia(), 6),
    ];

    println!("  First-Classification Latency:");
    println!(
        "  {:30} {:>8} {:>8} {:>8} {:>8} {:>10}",
        "Condition", "Mean ms", "SD ms", "Min ms", "Max ms", "µs/step"
    );
    println!(
        "  {:─>30} {:─>8} {:─>8} {:─>8} {:─>8} {:─>10}",
        "", "", "", "", "", ""
    );

    for r in &results {
        if r.latencies_ms.is_empty() {
            println!(
                "  {:30} {:>8} {:>8} {:>8} {:>8} {:>10.3}",
                r.name,
                "N/A",
                "N/A",
                "N/A",
                "N/A",
                r.mean_us_per_step()
            );
        } else {
            println!(
                "  {:30} {:>8.1} {:>8.3} {:>8.0} {:>8.0} {:>10.3}",
                r.name,
                r.mean_latency(),
                r.sd_latency(),
                r.min_latency(),
                r.max_latency(),
                r.mean_us_per_step()
            );
        }
    }

    // Throughput summary
    let avg_us = results.iter().map(|r| r.mean_us_per_step()).sum::<f64>() / results.len() as f64;
    let steps_per_sec = 1_000_000.0 / avg_us;
    println!(
        "\n  Average: {:.3} µs/step = {:.1}M steps/sec",
        avg_us,
        steps_per_sec / 1_000_000.0
    );
    println!(
        "  RAM: {} bytes (from metrics.json: 928)",
        CardiacDetector::new().memory_usage_bytes()
    );
}
