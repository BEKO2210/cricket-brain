use std::collections::VecDeque;

/// A delay synapse connecting two neurons with a fixed propagation delay.
///
/// Models the axonal delay lines found in the cricket auditory system.
/// Each synapse maintains a ring buffer that delays the signal by exactly
/// `delay_ms` timesteps before delivery.
///
/// Synapses can be excitatory (positive signal) or inhibitory (inverted signal).
#[derive(Debug, Clone)]
pub struct DelaySynapse {
    /// Source neuron index.
    pub from: usize,
    /// Target neuron index.
    pub to: usize,
    /// Propagation delay in milliseconds (timesteps).
    pub delay_ms: usize,
    /// If true, the synapse inverts the signal (inhibitory).
    pub inhibitory: bool,
    /// Ring buffer implementing the delay line.
    /// Length = delay_ms. The oldest element is at index 0.
    pub ring_buffer: VecDeque<f32>,
}

impl DelaySynapse {
    /// Creates a new delay synapse.
    ///
    /// # Arguments
    /// * `from` - Index of the source neuron
    /// * `to` - Index of the target neuron
    /// * `delay` - Propagation delay in ms (number of timesteps)
    /// * `inhibitory` - Whether this synapse inverts the signal
    ///
    /// # Example
    /// ```
    /// use cricket_brain::synapse::DelaySynapse;
    /// let syn = DelaySynapse::new(0, 1, 3, true);
    /// assert_eq!(syn.delay_ms, 3);
    /// assert!(syn.inhibitory);
    /// ```
    pub fn new(from: usize, to: usize, delay: usize, inhibitory: bool) -> Self {
        let mut ring_buffer = VecDeque::with_capacity(delay);
        for _ in 0..delay {
            ring_buffer.push_back(0.0);
        }
        DelaySynapse {
            from,
            to,
            delay_ms: delay,
            inhibitory,
            ring_buffer,
        }
    }

    /// Transmits a signal through the delay line.
    ///
    /// The signal is delayed by exactly `delay_ms` timesteps using a ring buffer.
    /// For inhibitory synapses, the output is negated.
    ///
    /// ```text
    /// output(t) = buffer[t - delay_ms]
    /// buffer[t] = signal  (pushed to back)
    /// ```
    ///
    /// # Arguments
    /// * `signal` - The input signal amplitude from the source neuron
    ///
    /// # Returns
    /// The delayed (and possibly inverted) signal.
    pub fn transmit(&mut self, signal: f32) -> f32 {
        // BUG #3 FIX: Read the oldest element BEFORE pop_front.
        // This ensures we read the signal from exactly delay_ms steps ago.
        let delayed_output = self.ring_buffer[0];

        // Shift the buffer: remove oldest, add newest
        self.ring_buffer.pop_front();
        self.ring_buffer.push_back(signal);

        if self.inhibitory {
            -delayed_output
        } else {
            delayed_output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synapse_delay() {
        let mut syn = DelaySynapse::new(0, 1, 3, false);
        // Push signal=1.0 through a 3ms delay
        assert_eq!(syn.transmit(1.0), 0.0); // t=0: output is 0 (initial)
        assert_eq!(syn.transmit(0.0), 0.0); // t=1: still 0
        assert_eq!(syn.transmit(0.0), 0.0); // t=2: still 0
        assert!((syn.transmit(0.0) - 1.0).abs() < f32::EPSILON); // t=3: delayed signal arrives
    }

    #[test]
    fn test_inhibitory_synapse() {
        let mut syn = DelaySynapse::new(0, 1, 1, true);
        syn.transmit(0.8);
        let out = syn.transmit(0.0);
        assert!((out - (-0.8)).abs() < f32::EPSILON);
    }
}
