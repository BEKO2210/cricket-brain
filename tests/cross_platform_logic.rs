// SPDX-License-Identifier: AGPL-3.0-only
use cricket_brain::brain::{BrainConfig, CricketBrain};

fn run_reference_trace() -> Vec<f32> {
    let mut brain =
        CricketBrain::new(BrainConfig::default()).expect("default config must be valid");
    let mut outputs = Vec::new();
    let input = [
        0.0, 4500.0, 4500.0, 4500.0, 0.0, 0.0, 4500.0, 4500.0, 0.0, 4500.0, 0.0, 0.0,
    ];
    for &f in &input {
        outputs.push(brain.step(f));
    }
    outputs
}

#[test]
fn deterministic_reference_trace_is_stable() {
    let run_a = run_reference_trace();
    let run_b = run_reference_trace();
    assert_eq!(run_a, run_b, "trace should be deterministic in-process");
}

#[test]
fn deterministic_trace_fingerprint_matches_golden() {
    let run = run_reference_trace();
    let fingerprint: Vec<i32> = run
        .iter()
        .map(|v| (v * 1_000_000.0).round() as i32)
        .collect();
    let golden = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(
        fingerprint, golden,
        "golden fingerprint changed; this can indicate cross-platform math drift"
    );
}
