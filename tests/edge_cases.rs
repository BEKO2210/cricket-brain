//! Edge-case tests for CricketBrain robustness.

use cricket_brain::brain::{BrainConfig, CricketBrain};
use cricket_brain::neuron::Neuron;
use cricket_brain::synapse::DelaySynapse;

#[test]
fn step_with_zero_frequency_produces_no_spike() {
    let mut brain = CricketBrain::new(Default::default()).unwrap();
    for _ in 0..100 {
        let out = brain.step(0.0);
        assert_eq!(out, 0.0, "silence must not produce spikes");
    }
}

#[test]
#[should_panic(expected = "input frequency must be positive")]
fn step_with_nan_panics_in_debug() {
    let mut brain = CricketBrain::new(Default::default()).unwrap();
    // NaN violates the debug_assert in Neuron::resonate — this is correct
    // defensive behavior: callers must validate input before stepping.
    brain.step(f32::NAN);
}

#[test]
#[should_panic(expected = "input frequency must be positive")]
fn step_with_infinity_panics_in_debug() {
    let mut brain = CricketBrain::new(Default::default()).unwrap();
    brain.step(f32::INFINITY);
}

#[test]
fn step_with_negative_freq_treated_as_silence() {
    let mut brain = CricketBrain::new(Default::default()).unwrap();
    // Negative frequencies are treated as silence (input_freq <= 0.0 branch)
    let out = brain.step(-100.0);
    assert_eq!(out, 0.0);
}

#[test]
fn neuron_with_large_delay_taps() {
    let n = Neuron::new(0, 4500.0, 500);
    assert_eq!(n.delay_taps, 500);
    assert!(n.history.len() <= 501);
}

#[test]
fn synapse_with_delay_one() {
    let mut syn = DelaySynapse::new(0, 1, 1, false);
    // First transmit returns the initial zero
    let out1 = syn.transmit(1.0);
    assert_eq!(out1, 0.0);
    // Second transmit returns the previously pushed 1.0
    let out2 = syn.transmit(0.5);
    assert_eq!(out2, 1.0);
}

#[test]
fn inhibitory_synapse_negates_signal() {
    let mut syn = DelaySynapse::new(0, 1, 1, true);
    syn.transmit(1.0);
    let out = syn.transmit(0.0);
    assert_eq!(out, -1.0, "inhibitory synapse must negate");
}

#[test]
fn scaled_brain_1000_neurons_does_not_panic() {
    let config = BrainConfig::scaled(1000, 3000);
    let mut brain = CricketBrain::new(config).unwrap();
    for _ in 0..10 {
        let _ = brain.step(4500.0);
    }
}

#[test]
fn brain_reset_clears_state() {
    let mut brain = CricketBrain::new(Default::default()).unwrap();
    for _ in 0..50 {
        brain.step(4500.0);
    }
    assert!(brain.time_step > 0);
    brain.reset();
    assert_eq!(brain.time_step, 0);
}

#[test]
fn determinism_same_seed_same_output() {
    let config = BrainConfig::default().with_seed(42);
    let mut brain1 = CricketBrain::new(config.clone()).unwrap();
    let mut brain2 = CricketBrain::new(config).unwrap();

    let signal = [4500.0, 0.0, 4500.0, 4500.0, 0.0, 4500.0, 0.0, 0.0];
    for &freq in signal.iter().cycle().take(200) {
        let o1 = brain1.step(freq);
        let o2 = brain2.step(freq);
        assert_eq!(
            o1.to_bits(),
            o2.to_bits(),
            "determinism violated at step {}",
            brain1.time_step
        );
    }
}

#[test]
fn memory_usage_within_embedded_limit() {
    let brain = CricketBrain::new(Default::default()).unwrap();
    let total = brain.memory_usage_bytes();
    assert!(
        total < 64 * 1024,
        "5-neuron circuit must fit in 64KB, got {total} bytes"
    );
}
