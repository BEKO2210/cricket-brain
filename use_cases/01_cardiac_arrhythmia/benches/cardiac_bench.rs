// SPDX-License-Identifier: AGPL-3.0-only
//! Criterion benchmarks for cardiac arrhythmia detector.
//! Date: 2026-04-10

use criterion::{criterion_group, criterion_main, Criterion};
use cricket_brain_cardiac::detector::CardiacDetector;
use cricket_brain_cardiac::ecg_signal;

fn bench_step_throughput(c: &mut Criterion) {
    let mut det = CardiacDetector::new();
    let stream = ecg_signal::normal_sinus().to_frequency_stream(10);

    c.bench_function("cardiac_step_normal_10cycles", |b| {
        b.iter(|| {
            det.reset();
            for &f in &stream {
                let _ = det.step(f);
            }
        })
    });
}

fn bench_classify_stream(c: &mut Criterion) {
    let beats = ecg_signal::from_csv("data/processed/sample_record.csv");
    let mut det = CardiacDetector::new();

    c.bench_function("cardiac_classify_150beats", |b| {
        b.iter(|| {
            det.classify_stream(&beats)
        })
    });
}

fn bench_tachy_detection(c: &mut Criterion) {
    let mut det = CardiacDetector::new();
    let stream = ecg_signal::tachycardia().to_frequency_stream(10);

    c.bench_function("cardiac_step_tachy_10cycles", |b| {
        b.iter(|| {
            det.reset();
            for &f in &stream {
                let _ = det.step(f);
            }
        })
    });
}

criterion_group!(benches, bench_step_throughput, bench_classify_stream, bench_tachy_detection);
criterion_main!(benches);
