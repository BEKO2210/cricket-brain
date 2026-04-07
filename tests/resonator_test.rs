// SPDX-License-Identifier: AGPL-3.0-only
use cricket_brain::neuron::Neuron;

#[test]
fn gaussian_perfect_match() {
    // At 0% frequency deviation, match_strength should be ~1.0
    // and amplitude should increase.
    let mut neuron = Neuron::new(0, 4500.0, 4);
    let amp = neuron.resonate(4500.0, 0.5);
    // match = exp(0) = 1.0, amplitude = 0.0 + 1.0 * 0.3 = 0.3
    assert!(
        (amp - 0.3).abs() < 0.01,
        "Expected ~0.3 at perfect match, got {amp}"
    );
}

#[test]
fn gaussian_20_percent_deviation() {
    // At 20% deviation: Δf/f₀ = 0.2, normalized = 0.2/0.1 = 2.0
    // match = exp(-4.0) ≈ 0.018 < 0.3 → should NOT resonate, amplitude decays
    let mut neuron = Neuron::new(0, 4500.0, 4);
    neuron.amplitude = 0.5; // pre-set to check decay
    let amp = neuron.resonate(4500.0 * 1.2, 0.5);
    // Amplitude should decay: 0.5 * 0.95 = 0.475 (less than before)
    assert!(
        amp < 0.5,
        "Expected amplitude to decay at 20% deviation, got {amp}"
    );
}

#[test]
fn gaussian_10_percent_boundary() {
    // At 10% deviation: normalized = 0.1/0.1 = 1.0
    // match = exp(-1.0) ≈ 0.368 > 0.3 → should still resonate
    let mut neuron = Neuron::new(0, 4500.0, 4);
    let amp = neuron.resonate(4500.0 * 1.1, 0.5);
    assert!(amp > 0.0, "Expected resonance at 10% deviation, got {amp}");
}

#[test]
fn decay_reduces_amplitude() {
    let mut neuron = Neuron::new(0, 4500.0, 4);
    neuron.amplitude = 0.8;
    neuron.decay();
    assert!(
        (neuron.amplitude - 0.76).abs() < 0.01,
        "Expected 0.8 * 0.95 = 0.76, got {}",
        neuron.amplitude
    );
}

#[test]
fn phase_decays_when_not_resonating() {
    // BUG #1 test: phase should decay in the else branch
    let mut neuron = Neuron::new(0, 4500.0, 4);
    neuron.phase = 0.5;
    neuron.amplitude = 0.5;
    // Feed a far-off frequency to trigger the else branch
    neuron.resonate(1000.0, 0.0);
    assert!(
        neuron.phase < 0.5,
        "Phase should decay when not resonating, got {}",
        neuron.phase
    );
}
