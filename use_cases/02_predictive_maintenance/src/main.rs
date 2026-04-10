// SPDX-License-Identifier: AGPL-3.0-only
//! Bearing fault detection demo.

use cricket_brain_bearings::detector::BearingDetector;
use cricket_brain_bearings::vibration_signal;
use std::time::Instant;

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
    println!("  Latency: {:.3} us/step | RAM: {} bytes ({} neurons)",
             us_per_step, det.memory_usage_bytes(), det.total_neurons());

    if classifications.is_empty() {
        println!("  No classifications\n");
        return;
    }

    for (i, (fault, conf, step)) in classifications.iter().enumerate() {
        println!("  Window {}: {} | Confidence={:.2} | Step={}", i + 1, fault, conf, step);
    }

    let last = &classifications[classifications.len() - 1];
    println!("  Final: {} (Conf={:.2})\n", last.0, last.1);
}

fn main() {
    println!("=== CricketBrain Bearing Fault Detection ===");
    println!("=== CWRU SKF 6205-2RS — 4-channel ResonatorBank ===\n");

    let mut det = BearingDetector::new();

    run_scenario("Normal (no fault)", &vibration_signal::normal_vibration(500), &mut det);
    run_scenario("Outer Race Fault (BPFO 107 Hz)", &vibration_signal::outer_race_fault(500), &mut det);
    run_scenario("Inner Race Fault (BPFI 162 Hz)", &vibration_signal::inner_race_fault(500), &mut det);
    run_scenario("Ball Defect (BSF 69 Hz)", &vibration_signal::ball_fault(500), &mut det);

    // Mixed scenario: normal → fault transition
    println!("--- Mixed: Normal → Outer Race Fault Onset ---");
    det.reset();
    let mut signal = vibration_signal::normal_vibration(300);
    signal.extend(vibration_signal::outer_race_fault(300));

    let mut beat_num = 0;
    for &freq in &signal {
        if let Some(fault) = det.step(freq) {
            beat_num += 1;
            println!("  Window {}: {} | Confidence={:.2}", beat_num, fault, det.confidence());
        }
    }
    println!();
}
