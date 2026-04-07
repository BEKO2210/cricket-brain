// SPDX-License-Identifier: AGPL-3.0-only
use cricket_brain::neuron::Neuron;

#[test]
fn no_fire_before_delay_filled() {
    // With delay_taps=4, the neuron should NOT fire before 4ms of stable signal,
    // because the history buffer's oldest element is still 0.0.
    let mut neuron = Neuron::new(0, 4500.0, 4);

    // Feed 3 timesteps of strong signal — not enough for coincidence
    for _ in 0..3 {
        neuron.resonate(4500.0, 0.5);
    }

    assert!(
        !neuron.check_coincidence(),
        "Should NOT fire before delay_taps ({}) timesteps of stable signal. \
         amplitude={}, history[0]={}",
        neuron.delay_taps,
        neuron.amplitude,
        neuron.history[0],
    );
}

#[test]
fn fires_after_delay_filled() {
    // After enough timesteps with strong signal, both current and delayed
    // amplitude should exceed threshold, triggering coincidence.
    let mut neuron = Neuron::new(0, 4500.0, 4);
    neuron.threshold = 0.5; // lower threshold to make test reliable

    // Feed many timesteps of strong signal to fill history
    for _ in 0..20 {
        neuron.resonate(4500.0, 0.5);
    }

    assert!(
        neuron.check_coincidence(),
        "Should fire after sustained signal. amplitude={}, history[0]={}, threshold={}",
        neuron.amplitude,
        neuron.history[0],
        neuron.threshold,
    );
}

#[test]
fn bug4_reads_oldest_not_newest() {
    // Verify BUG #4 fix: check_coincidence reads history[0] (oldest),
    // not history[delay_taps] (newest/current).
    let mut neuron = Neuron::new(0, 4500.0, 4);

    // Fill history with zeros, then set high amplitude
    neuron.amplitude = 0.9;
    // history is [0, 0, 0, 0, 0] — oldest is 0.0
    // Even though amplitude > threshold, delayed (oldest) is 0.0 < threshold*0.8
    assert!(
        !neuron.check_coincidence(),
        "BUG #4: Should read oldest history element (0.0), not current. \
         history[0]={}, amplitude={}",
        neuron.history[0],
        neuron.amplitude,
    );
}
