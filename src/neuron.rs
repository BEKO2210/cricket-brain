use std::collections::VecDeque;

/// A biomorphic neuron modeled after the auditory interneurons in the cricket
/// nervous system (Münster model). Each neuron has a characteristic eigenfrequency
/// and acts as a resonator with phase-locking capability.
///
/// The neuron uses Gaussian tuning for frequency selectivity and maintains a
/// history buffer for coincidence detection across delay lines.
#[derive(Debug, Clone)]
pub struct Neuron {
    /// Unique identifier for this neuron in the network.
    pub id: usize,
    /// Eigenfrequency (Hz) this neuron is tuned to.
    pub eigenfreq: f32,
    /// Current oscillation phase (0.0 to 1.0).
    pub phase: f32,
    /// Current activation amplitude (0.0 to 1.0).
    pub amplitude: f32,
    /// Firing threshold — neuron fires when amplitude exceeds this value.
    pub threshold: f32,
    /// Number of timesteps to look back for coincidence detection.
    pub delay_taps: usize,
    /// Ring buffer storing amplitude history for coincidence detection.
    /// Capacity = delay_taps + 1, so index 0 is the oldest sample.
    pub history: VecDeque<f32>,
}

impl Neuron {
    /// Creates a new neuron tuned to the given frequency.
    ///
    /// # Arguments
    /// * `id` - Unique neuron identifier
    /// * `freq` - Eigenfrequency in Hz (e.g., 4500.0 for cricket carrier)
    /// * `delay_ms` - Delay tap length in milliseconds for coincidence detection
    ///
    /// # Example
    /// ```
    /// use cricket_brain::neuron::Neuron;
    /// let an1 = Neuron::new(0, 4500.0, 4);
    /// assert_eq!(an1.eigenfreq, 4500.0);
    /// ```
    pub fn new(id: usize, freq: f32, delay_ms: usize) -> Self {
        let capacity = delay_ms + 1;
        let mut history = VecDeque::with_capacity(capacity);
        for _ in 0..capacity {
            history.push_back(0.0);
        }
        Neuron {
            id,
            eigenfreq: freq,
            phase: 0.0,
            amplitude: 0.0,
            threshold: 0.7,
            delay_taps: delay_ms,
            history,
        }
    }

    /// Resonance function using Gaussian frequency tuning.
    ///
    /// Computes match strength between the input frequency and the neuron's
    /// eigenfrequency using a Gaussian filter:
    ///
    /// ```text
    /// match = exp( -(Δf / f₀ / w)² )
    /// ```
    ///
    /// where `Δf = |input_freq - eigenfreq|`, `f₀ = eigenfreq`, and `w = 0.1`
    /// (10% bandwidth).
    ///
    /// If `match > 0.3`:
    /// - Amplitude increases: `A(t+1) = min(A(t) + match * 0.3, 1.0)`
    /// - Phase locks: `φ(t+1) = φ(t) + (φ_in - φ(t)) * 0.1`
    ///
    /// If `match <= 0.3`:
    /// - Amplitude decays: `A(t+1) = A(t) * 0.95`
    /// - Phase drifts toward zero: `φ(t+1) = φ(t) * 0.98`
    ///
    /// # Arguments
    /// * `input_freq` - The incoming signal frequency in Hz
    /// * `input_phase` - The incoming signal phase (0.0 to 1.0)
    ///
    /// # Returns
    /// The current amplitude after update.
    pub fn resonate(&mut self, input_freq: f32, input_phase: f32) -> f32 {
        // Gaussian tuning curve: match = exp(-(Δf / f₀ / w)²)
        let delta_f = (input_freq - self.eigenfreq).abs();
        let width = 0.1; // 10% bandwidth
        let normalized = delta_f / self.eigenfreq / width;
        let match_strength = (-normalized * normalized).exp();

        if match_strength > 0.3 {
            // Resonance: amplitude grows, phase locks
            // A(t+1) = min(A(t) + match * 0.3, 1.0)
            self.amplitude = (self.amplitude + match_strength * 0.3).min(1.0);
            // φ(t+1) = φ(t) + (φ_in - φ(t)) * 0.1
            self.phase += (input_phase - self.phase) * 0.1;
        } else {
            // No resonance: decay
            // A(t+1) = A(t) * 0.95
            self.amplitude *= 0.95;
            // BUG #1 FIX: Phase also decays when not resonating
            // φ(t+1) = φ(t) * 0.98
            self.phase *= 0.98;
        }

        // Update history ring buffer for coincidence detection
        if self.history.len() == self.delay_taps + 1 {
            self.history.pop_front();
        }
        self.history.push_back(self.amplitude);

        self.amplitude
    }

    /// Checks if this neuron should fire based on coincidence detection.
    ///
    /// A neuron fires when both the current amplitude AND the delayed amplitude
    /// (from `delay_taps` timesteps ago) exceed their respective thresholds:
    ///
    /// ```text
    /// fire = (A(t) > θ) ∧ (A(t - τ) > θ * 0.8)
    /// ```
    ///
    /// where `θ` is the firing threshold and `τ` is the delay tap length.
    ///
    /// # Returns
    /// `true` if the coincidence condition is met.
    pub fn check_coincidence(&self) -> bool {
        // BUG #4 FIX: Read index 0 (oldest element) instead of index delay_taps
        // (which would be the newest/current element).
        // self.history[0] is the amplitude from delay_taps timesteps ago.
        let delayed = self.history[0];
        self.amplitude > self.threshold && delayed > self.threshold * 0.8
    }

    /// Passive decay applied each timestep when no input is present.
    ///
    /// ```text
    /// A(t+1) = A(t) * 0.95
    /// φ(t+1) = φ(t) * 0.98
    /// ```
    pub fn decay(&mut self) {
        self.amplitude *= 0.95;
        self.phase *= 0.98;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neuron_creation() {
        let n = Neuron::new(0, 4500.0, 4);
        assert_eq!(n.id, 0);
        assert_eq!(n.eigenfreq, 4500.0);
        assert_eq!(n.history.len(), 5); // delay_taps + 1
    }

    #[test]
    fn test_resonance_at_eigenfreq() {
        let mut n = Neuron::new(0, 4500.0, 4);
        let amp = n.resonate(4500.0, 0.5);
        assert!(amp > 0.0);
    }

    #[test]
    fn test_no_resonance_far_freq() {
        let mut n = Neuron::new(0, 4500.0, 4);
        n.amplitude = 0.5;
        let amp = n.resonate(1000.0, 0.5);
        assert!(amp < 0.5); // should decay
    }
}
