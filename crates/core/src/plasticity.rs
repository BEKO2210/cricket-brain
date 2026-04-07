// SPDX-License-Identifier: AGPL-3.0-only
//! Spike-Timing Dependent Plasticity (STDP) for adaptive synaptic learning.
//!
//! STDP adjusts synaptic weights based on the relative timing of pre- and
//! post-synaptic spikes:
//!
//! - **Pre before post** (causal): strengthen connection (LTP)
//! - **Post before pre** (anti-causal): weaken connection (LTD)
//!
//! The weight update follows an exponential decay window:
//!
//! ```text
//! Δw = η · exp(-|Δt| / τ)     if pre fires before post  (LTP)
//! Δw = -η · exp(-|Δt| / τ)    if post fires before pre  (LTD)
//! ```
//!
//! This module provides pure functions that operate on timestamps and weights
//! without any heap allocation, making it fully `no_std` compatible.

use crate::synapse::DelaySynapse;

/// Configuration for the STDP learning rule.
#[derive(Debug, Clone, Copy)]
pub struct StdpConfig {
    /// Learning rate (η). Typical range: 0.001 to 0.05.
    /// Higher values = faster learning but less stable.
    pub learning_rate: f32,
    /// Time constant (τ) in timesteps. Controls the width of the
    /// STDP window — how far apart pre/post spikes can be and still
    /// cause weight change. Typical range: 5 to 30.
    pub time_constant: f32,
    /// Minimum allowed weight (default: -2.0).
    pub weight_min: f32,
    /// Maximum allowed weight (default: 2.0).
    pub weight_max: f32,
}

impl Default for StdpConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            time_constant: 10.0,
            weight_min: -2.0,
            weight_max: 2.0,
        }
    }
}

impl StdpConfig {
    /// Create a new STDP config with the given learning rate.
    pub fn with_learning_rate(mut self, lr: f32) -> Self {
        self.learning_rate = lr;
        self
    }

    /// Set the time constant (STDP window width in timesteps).
    pub fn with_time_constant(mut self, tau: f32) -> Self {
        self.time_constant = tau;
        self
    }

    /// Set weight bounds.
    pub fn with_weight_bounds(mut self, min: f32, max: f32) -> Self {
        self.weight_min = min;
        self.weight_max = max;
        self
    }
}

/// Computes the STDP weight delta for a single synapse.
///
/// # Arguments
/// * `pre_spike_time` - Timestep when the pre-synaptic neuron last spiked
/// * `post_spike_time` - Timestep when the post-synaptic neuron last spiked
/// * `config` - STDP parameters
///
/// # Returns
/// The weight adjustment to apply (positive = potentiate, negative = depress).
/// Returns 0.0 if either neuron has not spiked (time = 0).
#[inline]
pub fn compute_stdp_delta(pre_spike_time: u32, post_spike_time: u32, config: &StdpConfig) -> f32 {
    if pre_spike_time == 0 || post_spike_time == 0 {
        return 0.0;
    }

    let dt = post_spike_time as f32 - pre_spike_time as f32;

    if dt.abs() < 0.001 {
        // Simultaneous spikes — no update
        return 0.0;
    }

    let decay = libm::expf(-dt.abs() / config.time_constant);

    if dt > 0.0 {
        // Pre before post → LTP (strengthen)
        config.learning_rate * decay
    } else {
        // Post before pre → LTD (weaken)
        -config.learning_rate * decay
    }
}

/// Applies the STDP rule to a synapse, updating its weight in place.
///
/// # Arguments
/// * `synapse` - The synapse to update
/// * `pre_spike_time` - Last spike time of the source neuron
/// * `post_spike_time` - Last spike time of the target neuron
/// * `config` - STDP parameters
///
/// # Returns
/// The weight delta that was applied.
#[inline]
pub fn apply_stdp(
    synapse: &mut DelaySynapse,
    pre_spike_time: u32,
    post_spike_time: u32,
    config: &StdpConfig,
) -> f32 {
    let delta = compute_stdp_delta(pre_spike_time, post_spike_time, config);
    if delta.abs() > 1e-8 {
        synapse.weight = (synapse.weight + delta).clamp(config.weight_min, config.weight_max);
    }
    delta
}
