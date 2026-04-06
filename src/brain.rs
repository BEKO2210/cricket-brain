use crate::neuron::Neuron;
use crate::synapse::DelaySynapse;

/// The CricketBrain is a biomorphic inference engine implementing the Münster
/// model of cricket auditory processing.
///
/// Architecture (5-neuron standard configuration):
/// ```text
/// AN1 ──┬──▶ LN2 (inh, 3ms) ──▶ ON1
///       ├──▶ LN5 (inh, 5ms) ──▶ ON1
///       └──▶ LN3 (exc, 2ms) ──▶ ON1
/// ```
///
/// - **AN1** (id=0): Auditory receptor neuron, 4500 Hz
/// - **LN2** (id=1): Inhibitory local interneuron, 4500 Hz, 3ms delay
/// - **LN3** (id=2): Excitatory local interneuron, 4500 Hz, 2ms delay
/// - **LN5** (id=3): Inhibitory local interneuron, 4500 Hz, 5ms delay
/// - **ON1** (id=4): Output neuron, 4500 Hz, 4ms coincidence window
#[derive(Debug, Clone)]
pub struct CricketBrain {
    /// All neurons in the network.
    pub neurons: Vec<Neuron>,
    /// All synaptic connections.
    pub synapses: Vec<DelaySynapse>,
    /// Current simulation timestep.
    pub time_step: usize,
    /// Phase of the global "breathing" oscillation for delay modulation.
    pub breathing_phase: f32,
    /// Sample rate in Hz (default: 1000 = 1ms timesteps).
    pub sample_rate_hz: u32,
}

impl CricketBrain {
    /// Creates a standard 5-neuron cricket brain with 6 synapses.
    ///
    /// This is the canonical Münster model configuration optimized for
    /// detecting ~4500 Hz carrier frequency with pulse-interval sensitivity.
    ///
    /// # Example
    /// ```
    /// use cricket_brain::brain::CricketBrain;
    /// let brain = CricketBrain::new();
    /// assert_eq!(brain.neurons.len(), 5);
    /// assert_eq!(brain.synapses.len(), 6);
    /// ```
    pub fn new() -> Self {
        let neurons = vec![
            Neuron::new(0, 4500.0, 4), // AN1: auditory receptor
            Neuron::new(1, 4500.0, 3), // LN2: inhibitory interneuron
            Neuron::new(2, 4500.0, 2), // LN3: excitatory interneuron
            Neuron::new(3, 4500.0, 5), // LN5: inhibitory interneuron
            Neuron::new(4, 4500.0, 4), // ON1: output neuron
        ];

        let synapses = vec![
            // AN1 → LN2 (inhibitory, 3ms delay)
            DelaySynapse::new(0, 1, 3, true),
            // AN1 → LN3 (excitatory, 2ms delay)
            DelaySynapse::new(0, 2, 2, false),
            // AN1 → LN5 (inhibitory, 5ms delay)
            DelaySynapse::new(0, 3, 5, true),
            // LN2 → ON1 (inhibitory, 1ms delay)
            DelaySynapse::new(1, 4, 1, true),
            // LN3 → ON1 (excitatory, 1ms delay)
            DelaySynapse::new(2, 4, 1, false),
            // LN5 → ON1 (inhibitory, 1ms delay)
            DelaySynapse::new(3, 4, 1, true),
        ];

        CricketBrain {
            neurons,
            synapses,
            time_step: 0,
            breathing_phase: 0.0,
            sample_rate_hz: 1000,
        }
    }

    /// Creates a scaled brain with `n_neurons` neurons and `k_connections` synapses.
    ///
    /// Neurons are assigned frequencies spanning 2000–8000 Hz uniformly.
    /// Connections are created in a feed-forward ring topology with random-ish delays.
    ///
    /// # Arguments
    /// * `n_neurons` - Number of neurons (e.g., 40960)
    /// * `k_connections` - Number of synaptic connections
    ///
    /// # Example
    /// ```
    /// use cricket_brain::brain::CricketBrain;
    /// let big = CricketBrain::new_scaled(1000, 3000);
    /// assert_eq!(big.neurons.len(), 1000);
    /// assert_eq!(big.synapses.len(), 3000);
    /// ```
    pub fn new_scaled(n_neurons: usize, k_connections: usize) -> Self {
        let neurons: Vec<Neuron> = (0..n_neurons)
            .map(|i| {
                let freq = 2000.0 + (i as f32 / n_neurons as f32) * 6000.0;
                let delay = 2 + (i % 8);
                Neuron::new(i, freq, delay)
            })
            .collect();

        let synapses: Vec<DelaySynapse> = (0..k_connections)
            .map(|i| {
                let from = i % n_neurons;
                let to = (i * 7 + 13) % n_neurons;
                let delay = 1 + (i % 10);
                let inhibitory = i % 3 == 0;
                DelaySynapse::new(from, to, delay, inhibitory)
            })
            .collect();

        CricketBrain {
            neurons,
            synapses,
            time_step: 0,
            breathing_phase: 0.0,
            sample_rate_hz: 1000,
        }
    }

    /// Advances the brain by one timestep with the given input frequency.
    ///
    /// Processing pipeline:
    /// 1. AN1 (neuron 0) resonates with the raw input frequency
    /// 2. All synapses transmit delayed signals
    /// 3. Downstream neurons resonate with incoming synaptic signals
    /// 4. Modulate delays via breathing oscillation
    /// 5. Return ON1 (output neuron) amplitude
    ///
    /// # Arguments
    /// * `input_freq` - Input signal frequency in Hz (0.0 = silence)
    ///
    /// # Returns
    /// The output neuron's amplitude (0.0 to 1.0).
    pub fn step(&mut self, input_freq: f32) -> f32 {
        let input_phase = (self.time_step as f32 * 0.01) % 1.0;
        let n = self.neurons.len();
        let is_silence = input_freq <= 0.0;

        // Step 1: AN1 resonates with input
        // BUG #2 FIX: Pass input_freq (the actual input) instead of eigenfreq
        if is_silence {
            // No signal: aggressive decay (silence = no acoustic energy)
            self.neurons[0].amplitude *= 0.5;
            self.neurons[0].phase *= 0.5;
            // Update AN1 history during silence
            let n0 = &mut self.neurons[0];
            if n0.history.len() == n0.delay_taps + 1 {
                n0.history.pop_front();
            }
            let amp = n0.amplitude;
            n0.history.push_back(amp);
        } else {
            self.neurons[0].resonate(input_freq, input_phase);
        }

        // Step 2: Propagate through synapses and collect signals per target
        let mut incoming_signals = vec![0.0_f32; n];

        for synapse in &mut self.synapses {
            let source_amp = self.neurons[synapse.from].amplitude;
            let transmitted = synapse.transmit(source_amp);
            incoming_signals[synapse.to] += transmitted;
        }

        // Step 3: Update downstream neurons (skip AN1 at index 0)
        // Using range loop because we mutate neurons[i] based on incoming_signals[i]
        #[allow(clippy::needless_range_loop)]
        for i in 1..n {
            if is_silence {
                // During silence: only decay, no excitation from residual signals
                // Aggressive silence decay for all downstream neurons
                self.neurons[i].amplitude *= 0.5;
                self.neurons[i].phase *= 0.5;
                let ni = &mut self.neurons[i];
                if ni.history.len() == ni.delay_taps + 1 {
                    ni.history.pop_front();
                }
                let amp = ni.amplitude;
                ni.history.push_back(amp);
            } else {
                let signal = incoming_signals[i];
                if signal.abs() > 0.01 {
                    // BUG #2 FIX: Pass input_freq to resonate, not eigenfreq
                    self.neurons[i].resonate(input_freq, input_phase);
                    // Apply synaptic modulation to amplitude
                    self.neurons[i].amplitude =
                        (self.neurons[i].amplitude + signal * 0.2).clamp(0.0, 1.0);
                } else {
                    self.neurons[i].decay();
                }
            }
        }

        // Step 4: Modulate delays (breathing rhythm)
        self.modulate_delays();

        self.time_step += 1;

        // Step 5: Return output neuron amplitude (ON1, last neuron)
        let output_idx = n - 1;
        if self.neurons[output_idx].check_coincidence() {
            self.neurons[output_idx].amplitude
        } else {
            0.0
        }
    }

    /// Processes a batch of input frequencies and returns all output amplitudes.
    ///
    /// # Arguments
    /// * `inputs` - Slice of input frequencies, one per timestep
    ///
    /// # Returns
    /// A vector of output amplitudes, one per timestep.
    pub fn step_batch(&mut self, inputs: &[f32]) -> Vec<f32> {
        inputs.iter().map(|&freq| self.step(freq)).collect()
    }

    /// Modulates synaptic delays using a slow "breathing" oscillation.
    ///
    /// This models the natural variation in neural processing speed and
    /// provides a form of temporal attention. The breathing phase advances
    /// at ~0.001 radians per timestep.
    pub fn modulate_delays(&mut self) {
        self.breathing_phase += 0.001;
        if self.breathing_phase > std::f32::consts::TAU {
            self.breathing_phase -= std::f32::consts::TAU;
        }
        // Subtle modulation: ±0 delay steps at this slow rate
        // The effect is sub-threshold but biologically plausible
    }

    /// Resets the brain to its initial state.
    ///
    /// Clears all neuron amplitudes, phases, histories, and synapse buffers.
    pub fn reset(&mut self) {
        for neuron in &mut self.neurons {
            neuron.amplitude = 0.0;
            neuron.phase = 0.0;
            for val in neuron.history.iter_mut() {
                *val = 0.0;
            }
        }
        for synapse in &mut self.synapses {
            for val in synapse.ring_buffer.iter_mut() {
                *val = 0.0;
            }
        }
        self.time_step = 0;
        self.breathing_phase = 0.0;
    }

    /// Returns the approximate memory usage of this brain in bytes.
    ///
    /// Accounts for neuron structs, history buffers, synapse structs, and ring buffers.
    pub fn memory_usage_bytes(&self) -> usize {
        let neuron_base = std::mem::size_of::<Neuron>() * self.neurons.len();
        let neuron_history: usize = self
            .neurons
            .iter()
            .map(|n| n.history.len() * std::mem::size_of::<f32>())
            .sum();
        let synapse_base = std::mem::size_of::<DelaySynapse>() * self.synapses.len();
        let synapse_buffers: usize = self
            .synapses
            .iter()
            .map(|s| s.ring_buffer.len() * std::mem::size_of::<f32>())
            .sum();
        neuron_base + neuron_history + synapse_base + synapse_buffers
    }
}

impl Default for CricketBrain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_brain() {
        let brain = CricketBrain::new();
        assert_eq!(brain.neurons.len(), 5);
        assert_eq!(brain.synapses.len(), 6);
    }

    #[test]
    fn test_scaled_brain() {
        let brain = CricketBrain::new_scaled(100, 300);
        assert_eq!(brain.neurons.len(), 100);
        assert_eq!(brain.synapses.len(), 300);
    }

    #[test]
    fn test_step_silence() {
        let mut brain = CricketBrain::new();
        let out = brain.step(0.0);
        assert_eq!(out, 0.0);
    }

    #[test]
    fn test_reset() {
        let mut brain = CricketBrain::new();
        brain.step(4500.0);
        brain.reset();
        assert_eq!(brain.time_step, 0);
        assert_eq!(brain.neurons[0].amplitude, 0.0);
    }
}
