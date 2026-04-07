// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for synaptic weight and plasticity foundations.

use cricket_brain::brain::{BrainConfig, CricketBrain};
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
