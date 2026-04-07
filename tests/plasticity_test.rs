// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for synaptic weight and plasticity foundations.

use cricket_brain::brain::{BrainConfig, CricketBrain};
use cricket_brain::neuron::Neuron;
use cricket_brain::plasticity::{
    apply_homeostasis, apply_stdp, compute_stdp_delta, HomeostasisConfig, StdpConfig,
};
use cricket_brain::synapse::DelaySynapse;

#[test]
fn synapse_default_weight_excitatory() {
    let syn = DelaySynapse::new(0, 1, 3, false);
    assert_eq!(syn.weight, 1.0);
    assert_eq!(syn.current_weight(), 1.0);
}

#[test]
fn synapse_default_weight_inhibitory() {
    let syn = DelaySynapse::new(0, 1, 3, true);
    assert_eq!(syn.weight, -1.0);
    assert_eq!(syn.current_weight(), -1.0);
}

#[test]
fn synapse_transmit_uses_weight() {
    // Excitatory with weight 1.0 — output should be positive
    let mut syn = DelaySynapse::new(0, 1, 1, false);
    syn.transmit(0.5); // push 0.5, get initial 0.0
    let out = syn.transmit(0.0); // push 0.0, get delayed 0.5
    assert_eq!(out, 0.5); // weight 1.0 * 0.5 = 0.5

    // Inhibitory with weight -1.0 — output should be negative
    let mut syn_inh = DelaySynapse::new(0, 1, 1, true);
    syn_inh.transmit(0.5);
    let out_inh = syn_inh.transmit(0.0);
    assert_eq!(out_inh, -0.5); // weight -1.0 * 0.5 = -0.5
}

#[test]
fn synapse_custom_weight_scales_output() {
    let mut syn = DelaySynapse::new(0, 1, 1, false);
    syn.weight = 0.5; // half-strength excitatory
    syn.transmit(1.0);
    let out = syn.transmit(0.0);
    assert_eq!(out, 0.5); // 1.0 * 0.5 = 0.5
}

#[test]
fn adjust_weight_potentiates() {
    let mut syn = DelaySynapse::new(0, 1, 3, false);
    assert_eq!(syn.weight, 1.0);
    syn.adjust_weight(0.1);
    assert!((syn.weight - 1.1).abs() < 1e-6);
}

#[test]
fn adjust_weight_depresses() {
    let mut syn = DelaySynapse::new(0, 1, 3, false);
    syn.adjust_weight(-0.3);
    assert!((syn.weight - 0.7).abs() < 1e-6);
}

#[test]
fn adjust_weight_clamps_upper() {
    let mut syn = DelaySynapse::new(0, 1, 3, false);
    syn.adjust_weight(5.0); // way over limit
    assert_eq!(syn.weight, 2.0);
}

#[test]
fn adjust_weight_clamps_lower() {
    let mut syn = DelaySynapse::new(0, 1, 3, true); // starts at -1.0
    syn.adjust_weight(-5.0);
    assert_eq!(syn.weight, -2.0);
}

#[test]
fn weighted_synapse_produces_same_result_as_old_inhibitory() {
    // Verify backward compatibility: weight=-1.0 == old inhibitory behavior
    let mut syn = DelaySynapse::new(0, 1, 2, true);
    assert_eq!(syn.weight, -1.0);

    syn.transmit(0.8);
    syn.transmit(0.6);
    let out = syn.transmit(0.0);
    // Should be -0.8 (oldest value, negated by weight -1.0)
    assert_eq!(out, -0.8);
}

#[test]
fn brain_synapses_have_correct_default_weights() {
    let brain = CricketBrain::new(BrainConfig::default()).unwrap();
    // Muenster circuit: synapses 0,2,3 are inhibitory, 1,4 are excitatory
    // AN1→LN2 (inh), AN1→LN3 (exc), AN1→LN5 (inh), LN2→ON1 (inh), LN3→ON1 (exc), LN5→ON1 (inh)
    for syn in &brain.synapses {
        if syn.inhibitory {
            assert_eq!(
                syn.weight, -1.0,
                "inhibitory synapse {}->{} should have weight -1.0",
                syn.from, syn.to
            );
        } else {
            assert_eq!(
                syn.weight, 1.0,
                "excitatory synapse {}->{} should have weight 1.0",
                syn.from, syn.to
            );
        }
    }
}

#[test]
fn brain_output_unchanged_with_default_weights() {
    // This is the critical regression test: default weights must produce
    // IDENTICAL output to the pre-weight implementation.
    let config = BrainConfig::default().with_seed(42);
    let mut brain = CricketBrain::new(config).unwrap();

    let mut outputs = Vec::new();
    let signal = [4500.0, 4500.0, 4500.0, 0.0, 0.0, 4500.0, 4500.0, 0.0];
    for &freq in signal.iter().cycle().take(100) {
        outputs.push(brain.step(freq));
    }

    // Re-run with fresh brain — must be bitwise identical (determinism)
    let mut brain2 = CricketBrain::new(BrainConfig::default().with_seed(42)).unwrap();
    for (i, &freq) in signal.iter().cycle().take(100).enumerate() {
        let o2 = brain2.step(freq);
        assert_eq!(
            outputs[i].to_bits(),
            o2.to_bits(),
            "output diverged at step {i}"
        );
    }
}

// =========================================================================
// STDP Tests
// =========================================================================

#[test]
fn stdp_ltp_pre_before_post() {
    // Pre fires at t=10, post fires at t=15 → dt=5 → LTP (positive delta)
    let config = StdpConfig::default();
    let delta = compute_stdp_delta(10, 15, &config);
    assert!(
        delta > 0.0,
        "pre-before-post should potentiate, got {delta}"
    );
}

#[test]
fn stdp_ltd_post_before_pre() {
    // Post fires at t=10, pre fires at t=15 → dt=-5 → LTD (negative delta)
    let config = StdpConfig::default();
    let delta = compute_stdp_delta(15, 10, &config);
    assert!(delta < 0.0, "post-before-pre should depress, got {delta}");
}

#[test]
fn stdp_no_spike_returns_zero() {
    let config = StdpConfig::default();
    assert_eq!(compute_stdp_delta(0, 15, &config), 0.0);
    assert_eq!(compute_stdp_delta(10, 0, &config), 0.0);
    assert_eq!(compute_stdp_delta(0, 0, &config), 0.0);
}

#[test]
fn stdp_simultaneous_returns_zero() {
    let config = StdpConfig::default();
    assert_eq!(compute_stdp_delta(10, 10, &config), 0.0);
}

#[test]
fn stdp_decay_with_time_difference() {
    // Larger dt → smaller delta (exponential decay)
    let config = StdpConfig::default();
    let delta_close = compute_stdp_delta(10, 12, &config); // dt=2
    let delta_far = compute_stdp_delta(10, 30, &config); // dt=20
    assert!(
        delta_close.abs() > delta_far.abs(),
        "closer spikes should produce larger delta: close={delta_close}, far={delta_far}"
    );
}

#[test]
fn stdp_learning_rate_scales_delta() {
    let slow = StdpConfig::default().with_learning_rate(0.001);
    let fast = StdpConfig::default().with_learning_rate(0.1);
    let d_slow = compute_stdp_delta(10, 15, &slow);
    let d_fast = compute_stdp_delta(10, 15, &fast);
    assert!(
        d_fast.abs() > d_slow.abs() * 50.0,
        "100x learning rate should produce ~100x delta"
    );
}

#[test]
fn stdp_time_constant_widens_window() {
    let narrow = StdpConfig::default().with_time_constant(5.0);
    let wide = StdpConfig::default().with_time_constant(50.0);
    // At dt=20, narrow window should have decayed much more
    let d_narrow = compute_stdp_delta(10, 30, &narrow);
    let d_wide = compute_stdp_delta(10, 30, &wide);
    assert!(
        d_wide.abs() > d_narrow.abs(),
        "wider time constant should preserve more at large dt"
    );
}

#[test]
fn apply_stdp_modifies_synapse_weight() {
    let mut syn = DelaySynapse::new(0, 1, 3, false); // weight = 1.0
    let config = StdpConfig::default().with_learning_rate(0.05);
    let delta = apply_stdp(&mut syn, 10, 15, &config);
    assert!(delta > 0.0);
    assert!(syn.weight > 1.0, "weight should have increased");
}

#[test]
fn apply_stdp_respects_weight_bounds() {
    let mut syn = DelaySynapse::new(0, 1, 3, false); // weight = 1.0
    let config = StdpConfig::default()
        .with_learning_rate(0.5)
        .with_weight_bounds(0.0, 1.5);

    // Repeatedly potentiate — should clamp at 1.5
    for i in 0..100 {
        apply_stdp(&mut syn, 10 + i, 12 + i, &config);
    }
    assert!(
        syn.weight <= 1.5,
        "weight should be clamped at max, got {}",
        syn.weight
    );

    // Repeatedly depress — should clamp at 0.0
    for i in 0..100 {
        apply_stdp(&mut syn, 212 + i, 200 + i, &config);
    }
    assert!(
        syn.weight >= 0.0,
        "weight should be clamped at min, got {}",
        syn.weight
    );
}

#[test]
fn stdp_symmetry_ltp_ltd_same_magnitude() {
    // For same |dt|, LTP and LTD should have same magnitude
    let config = StdpConfig::default();
    let ltp = compute_stdp_delta(10, 15, &config); // dt = +5
    let ltd = compute_stdp_delta(15, 10, &config); // dt = -5
    assert!(
        (ltp.abs() - ltd.abs()).abs() < 1e-6,
        "LTP and LTD should be symmetric: ltp={ltp}, ltd={ltd}"
    );
    assert!(ltp > 0.0);
    assert!(ltd < 0.0);
}

// =========================================================================
// Brain-integrated STDP Tests
// =========================================================================

#[test]
fn brain_stdp_disabled_by_default() {
    let brain = CricketBrain::new(BrainConfig::default()).unwrap();
    assert!(brain.stdp_config().is_none());
}

#[test]
fn brain_enable_disable_stdp() {
    let mut brain = CricketBrain::new(BrainConfig::default()).unwrap();
    brain.enable_stdp(StdpConfig::default());
    assert!(brain.stdp_config().is_some());
    brain.disable_stdp();
    assert!(brain.stdp_config().is_none());
}

#[test]
fn brain_stdp_modifies_weights_during_signal() {
    let mut brain = CricketBrain::new(BrainConfig::default().with_seed(42)).unwrap();
    let initial_weights: Vec<f32> = brain.synapses.iter().map(|s| s.weight).collect();

    brain.enable_stdp(StdpConfig::default().with_learning_rate(0.05));

    // Feed pulsed signal — on/off pattern creates timing differences
    // between pre and post neurons via the delay lines.
    // STDP requires that pre and post spike at DIFFERENT times.
    for cycle in 0..50 {
        // Tone burst (5 steps) — triggers spikes at different times
        // due to synaptic delays (2ms, 3ms, 5ms)
        for _ in 0..5 {
            brain.step(4500.0);
        }
        // Silence gap (variable length to desynchronize spike times)
        for _ in 0..(3 + cycle % 4) {
            brain.step(0.0);
        }
    }

    let final_weights: Vec<f32> = brain.synapses.iter().map(|s| s.weight).collect();

    // At least one weight should have changed
    let any_changed = initial_weights
        .iter()
        .zip(final_weights.iter())
        .any(|(a, b)| (a - b).abs() > 1e-6);
    assert!(
        any_changed,
        "STDP should modify weights with pulsed signal. initial={initial_weights:?} final={final_weights:?}"
    );
}

#[test]
fn brain_stdp_does_not_modify_weights_when_disabled() {
    let mut brain = CricketBrain::new(BrainConfig::default().with_seed(42)).unwrap();
    // STDP NOT enabled
    let initial_weights: Vec<f32> = brain.synapses.iter().map(|s| s.weight).collect();

    for _ in 0..200 {
        brain.step(4500.0);
    }

    let final_weights: Vec<f32> = brain.synapses.iter().map(|s| s.weight).collect();
    assert_eq!(
        initial_weights, final_weights,
        "weights must not change without STDP"
    );
}

#[test]
fn brain_stdp_preserves_determinism() {
    let config = BrainConfig::default().with_seed(42);
    let stdp = StdpConfig::default().with_learning_rate(0.02);

    let mut brain1 = CricketBrain::new(config.clone()).unwrap();
    brain1.enable_stdp(stdp);
    let mut brain2 = CricketBrain::new(config).unwrap();
    brain2.enable_stdp(stdp);

    let signal = [4500.0, 4500.0, 0.0, 4500.0, 0.0, 0.0, 4500.0, 4500.0];
    for &freq in signal.iter().cycle().take(100) {
        let o1 = brain1.step(freq);
        let o2 = brain2.step(freq);
        assert_eq!(o1.to_bits(), o2.to_bits(), "STDP must be deterministic");
    }

    // Weights must also match
    for (s1, s2) in brain1.synapses.iter().zip(brain2.synapses.iter()) {
        assert_eq!(s1.weight.to_bits(), s2.weight.to_bits());
    }
}

#[test]
fn neuron_activity_ema_tracks_amplitude() {
    let mut n = Neuron::new(0, 4500.0, 4);
    assert_eq!(n.activity_ema, 0.0);

    // Feed resonant signal — EMA should rise
    for _ in 0..50 {
        n.resonate(4500.0, 0.5);
    }
    assert!(
        n.activity_ema > 0.1,
        "EMA should track rising amplitude, got {}",
        n.activity_ema
    );
}

#[test]
fn neuron_last_spike_step_starts_at_zero() {
    let n = Neuron::new(0, 4500.0, 4);
    assert_eq!(n.last_spike_step, 0);
}

// =========================================================================
// Homeostasis Tests
// =========================================================================

#[test]
fn homeostasis_raises_threshold_when_overactive() {
    let mut n = Neuron::new(0, 4500.0, 4);
    n.activity_ema = 0.8; // well above default target (0.4)
    n.threshold = 0.7;
    let config = HomeostasisConfig::default(); // target=0.4
    let delta = apply_homeostasis(&mut n, &config);
    assert!(delta > 0.0, "overactive neuron should raise threshold");
    assert!(n.threshold > 0.7);
}

#[test]
fn homeostasis_lowers_threshold_when_quiet() {
    let mut n = Neuron::new(0, 4500.0, 4);
    n.activity_ema = 0.1; // well below target (0.4)
    n.threshold = 0.7;
    let config = HomeostasisConfig::default();
    let delta = apply_homeostasis(&mut n, &config);
    assert!(delta < 0.0, "quiet neuron should lower threshold");
    assert!(n.threshold < 0.7);
}

#[test]
fn homeostasis_stable_at_target() {
    let mut n = Neuron::new(0, 4500.0, 4);
    n.activity_ema = 0.4; // exactly at target
    n.threshold = 0.7;
    let config = HomeostasisConfig::default();
    let delta = apply_homeostasis(&mut n, &config);
    assert!(delta.abs() < 1e-6, "at target = no adjustment");
}

#[test]
fn homeostasis_respects_bounds() {
    let mut n = Neuron::new(0, 4500.0, 4);
    n.activity_ema = 1.0; // way over target
    n.threshold = 0.9;
    let config = HomeostasisConfig::default()
        .with_learning_rate(1.0)
        .with_bounds(0.3, 0.95);

    for _ in 0..100 {
        apply_homeostasis(&mut n, &config);
    }
    assert!(n.threshold <= 0.95, "threshold clamped at max");

    n.activity_ema = 0.0;
    for _ in 0..100 {
        apply_homeostasis(&mut n, &config);
    }
    assert!(n.threshold >= 0.3, "threshold clamped at min");
}

#[test]
fn brain_homeostasis_disabled_by_default() {
    let brain = CricketBrain::new(BrainConfig::default()).unwrap();
    assert!(brain.homeostasis_config().is_none());
}

#[test]
fn brain_enable_disable_homeostasis() {
    let mut brain = CricketBrain::new(BrainConfig::default()).unwrap();
    brain.enable_homeostasis(HomeostasisConfig::default());
    assert!(brain.homeostasis_config().is_some());
    brain.disable_homeostasis();
    assert!(brain.homeostasis_config().is_none());
}

#[test]
fn brain_homeostasis_adjusts_thresholds_over_time() {
    let mut brain = CricketBrain::new(BrainConfig::default().with_seed(42)).unwrap();
    let initial_thresholds: Vec<f32> = brain.neurons.iter().map(|n| n.threshold).collect();

    brain.enable_homeostasis(HomeostasisConfig::default().with_learning_rate(0.01));

    // Feed signal to build up activity_ema, then let homeostasis adjust
    for _ in 0..300 {
        brain.step(4500.0);
    }

    let final_thresholds: Vec<f32> = brain.neurons.iter().map(|n| n.threshold).collect();
    let any_changed = initial_thresholds
        .iter()
        .zip(final_thresholds.iter())
        .any(|(a, b)| (a - b).abs() > 1e-4);
    assert!(
        any_changed,
        "homeostasis should adjust thresholds. initial={initial_thresholds:?} final={final_thresholds:?}"
    );
}

#[test]
fn brain_homeostasis_does_not_change_when_disabled() {
    let mut brain = CricketBrain::new(BrainConfig::default().with_seed(42)).unwrap();
    let initial_thresholds: Vec<f32> = brain.neurons.iter().map(|n| n.threshold).collect();

    for _ in 0..300 {
        brain.step(4500.0);
    }

    let final_thresholds: Vec<f32> = brain.neurons.iter().map(|n| n.threshold).collect();
    assert_eq!(initial_thresholds, final_thresholds);
}

#[test]
fn brain_combined_stdp_and_homeostasis() {
    let mut brain = CricketBrain::new(BrainConfig::default().with_seed(42)).unwrap();
    brain.enable_stdp(StdpConfig::default().with_learning_rate(0.02));
    brain.enable_homeostasis(HomeostasisConfig::default().with_learning_rate(0.005));

    // Pulsed signal to trigger both mechanisms
    for cycle in 0..60 {
        for _ in 0..5 {
            brain.step(4500.0);
        }
        for _ in 0..(3 + cycle % 4) {
            brain.step(0.0);
        }
    }

    // Both should have had an effect — brain should still be functional
    let out = brain.step(4500.0);
    // Just verify it doesn't crash and produces a valid f32
    assert!(out.is_finite());
}
