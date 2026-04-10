// SPDX-License-Identifier: AGPL-3.0-only
//! Cardiac arrhythmia pre-screening demo.
//!
//! Generates synthetic ECG waveforms (normal, tachycardia, bradycardia),
//! feeds them through the CardiacDetector, and reports rhythm classification
//! with BPM and confidence per beat.

use cricket_brain_cardiac::detector::CardiacDetector;
use cricket_brain_cardiac::ecg_signal;
use std::time::Instant;

fn run_scenario(
    label: &str,
    cycle: &ecg_signal::EcgCycle,
    n_cycles: usize,
    detector: &mut CardiacDetector,
) {
    detector.reset();
    let stream = cycle.to_frequency_stream(n_cycles);
    let expected_bpm = cycle.bpm();

    let t0 = Instant::now();
    let mut classifications = Vec::new();

    for &freq in &stream {
        if let Some(class) = detector.step(freq) {
            classifications.push((
                class,
                detector.bpm_estimate(),
                detector.confidence(),
                detector.steps_processed(),
            ));
        }
    }

    let elapsed = t0.elapsed();
    let us_per_step = elapsed.as_secs_f64() * 1_000_000.0 / stream.len() as f64;

    println!("--- {label} ({n_cycles} cycles, expected ~{expected_bpm:.0} BPM) ---");
    println!(
        "  Processed: {} steps in {:.3} ms ({:.3} us/step)",
        stream.len(),
        elapsed.as_secs_f64() * 1000.0,
        us_per_step
    );
    println!(
        "  RAM: {} bytes",
        detector.memory_usage_bytes()
    );

    if classifications.is_empty() {
        println!("  No classifications (insufficient beats)\n");
        return;
    }

    for (i, (class, bpm, conf, step)) in classifications.iter().enumerate() {
        println!(
            "  Beat {}: {} | BPM={:.0} | Confidence={:.2} | Step={}",
            i + 1,
            class,
            bpm,
            conf,
            step
        );
    }

    let last = &classifications[classifications.len() - 1];
    println!(
        "  Final: {} (BPM={:.0}, Conf={:.2})\n",
        last.0, last.1, last.2
    );
}

fn main() {
    println!("=== CricketBrain Cardiac Arrhythmia Pre-Screening ===");
    println!("=== NOT a medical device — research prototype only ===\n");

    let mut detector = CardiacDetector::new();

    run_scenario("Normal Sinus Rhythm", &ecg_signal::normal_sinus(), 5, &mut detector);
    run_scenario("Tachycardia", &ecg_signal::tachycardia(), 5, &mut detector);
    run_scenario("Bradycardia", &ecg_signal::bradycardia(), 5, &mut detector);

    // Mixed scenario: normal → tachy transition
    println!("--- Mixed: Normal → Tachycardia Transition ---");
    detector.reset();
    let mut stream = ecg_signal::normal_sinus().to_frequency_stream(4);
    stream.extend(ecg_signal::tachycardia().to_frequency_stream(4));

    let t0 = Instant::now();
    let mut beat_num = 0;
    for &freq in &stream {
        if let Some(class) = detector.step(freq) {
            beat_num += 1;
            println!(
                "  Beat {}: {} | BPM={:.0} | Confidence={:.2}",
                beat_num,
                class,
                detector.bpm_estimate(),
                detector.confidence()
            );
        }
    }
    let elapsed = t0.elapsed();
    println!(
        "  Total: {} steps in {:.3} ms\n",
        stream.len(),
        elapsed.as_secs_f64() * 1000.0
    );
}
