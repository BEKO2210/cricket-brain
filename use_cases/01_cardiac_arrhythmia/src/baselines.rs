// SPDX-License-Identifier: AGPL-3.0-only
//! Simple non-neuromorphic baselines for the cardiac rhythm-pattern
//! triage benchmark.
//!
//! These baselines exist for one reason: **a CricketBrain result only
//! means something if a trivially simple alternative cannot do the
//! same job**. Every claim about UC01's strength has to be measured
//! against *at least* the rule-based baselines below.
//!
//! All baselines:
//!
//! * consume the same ms-resolution frequency stream as
//!   [`crate::detector::CardiacDetector`],
//! * emit `(timestep, predicted RhythmClass)` tuples in the same shape,
//! * are pure rule-based code — no learning, no thresholds tuned per
//!   recording.
//!
//! Two baselines are provided:
//!
//! 1. **Threshold-burst baseline** — counts in-band bursts (the same
//!    QRS band CricketBrain resonates at) and reproduces the
//!    rate-regime classification logic (`>100 BPM`, `<60 BPM`,
//!    irregular if CV(RR) > 0.3). It is intentionally close to the
//!    detector's rule layer so it isolates the contribution of
//!    CricketBrain's coincidence gate.
//! 2. **Frequency-rule baseline** — splits the stream into 1-second
//!    windows, counts samples in the QRS band per window, and
//!    classifies the running mean as Brady / Normal / Tachy. It is
//!    deliberately simpler than (1) and demonstrates that even a
//!    primitive rule has real performance on synthetic data.

use crate::detector::RhythmClass;

/// One classification emitted by a baseline at a given timestep.
#[derive(Debug, Clone)]
pub struct BaselinePrediction {
    pub step: usize,
    pub class: RhythmClass,
    pub bpm: f32,
}

/// Configurable threshold-burst baseline.
#[derive(Debug, Clone)]
pub struct ThresholdBurstBaseline {
    /// Centre of the QRS-band gate (Hz).
    pub band_center: f32,
    /// Fractional bandwidth (e.g. 0.10 = ±10%).
    pub band_width: f32,
    /// Minimum consecutive in-band steps to count as a beat.
    pub min_burst_ms: usize,
    /// Refractory period (ms) — minimum spacing between successive beats.
    pub refractory_ms: usize,
    /// RR-window depth (number of beats kept for classification).
    pub rr_window: usize,
    /// CV(RR) threshold above which the rhythm is called Irregular.
    pub cv_irregular: f32,
}

impl Default for ThresholdBurstBaseline {
    fn default() -> Self {
        Self {
            band_center: 4500.0,
            band_width: 0.10,
            min_burst_ms: 2,
            refractory_ms: 150,
            rr_window: 8,
            cv_irregular: 0.3,
        }
    }
}

impl ThresholdBurstBaseline {
    fn in_band(&self, freq: f32) -> bool {
        if freq <= 0.0 {
            return false;
        }
        let dev = (freq - self.band_center).abs() / self.band_center;
        dev <= self.band_width
    }

    /// Run the baseline over a frequency stream, emitting one
    /// classification each time a *new* RR interval becomes available.
    pub fn run(&self, stream: &[f32]) -> Vec<BaselinePrediction> {
        let mut preds = Vec::new();
        let mut in_burst = false;
        let mut burst_len: usize = 0;
        let mut burst_start: usize = 0;
        let mut last_beat_step: usize = 0;
        let mut rr_intervals: std::collections::VecDeque<usize> =
            std::collections::VecDeque::with_capacity(self.rr_window + 1);

        for (i, &freq) in stream.iter().enumerate() {
            let inside = self.in_band(freq);
            let in_refractory =
                last_beat_step > 0 && i.saturating_sub(last_beat_step) < self.refractory_ms;

            if inside && !in_refractory {
                if !in_burst {
                    in_burst = true;
                    burst_len = 1;
                    burst_start = i;
                } else {
                    burst_len += 1;
                }
            } else if in_burst {
                let valid = burst_len >= self.min_burst_ms;
                if valid {
                    if last_beat_step > 0 {
                        let rr = burst_start.saturating_sub(last_beat_step);
                        if rr > 10 && rr < 3000 {
                            rr_intervals.push_back(rr);
                            if rr_intervals.len() > self.rr_window {
                                rr_intervals.pop_front();
                            }
                        }
                    }
                    last_beat_step = burst_start;

                    if rr_intervals.len() >= 2 {
                        let mean =
                            rr_intervals.iter().sum::<usize>() as f32 / rr_intervals.len() as f32;
                        let bpm = 60_000.0 / mean;
                        let var = rr_intervals
                            .iter()
                            .map(|&rr| {
                                let d = rr as f32 - mean;
                                d * d
                            })
                            .sum::<f32>()
                            / rr_intervals.len() as f32;
                        let cv = var.sqrt() / mean;

                        let class = if cv > self.cv_irregular {
                            RhythmClass::Irregular
                        } else if bpm > 100.0 {
                            RhythmClass::Tachycardia
                        } else if bpm < 60.0 {
                            RhythmClass::Bradycardia
                        } else {
                            RhythmClass::NormalSinus
                        };

                        preds.push(BaselinePrediction {
                            step: i,
                            class,
                            bpm,
                        });
                    }
                }
                in_burst = false;
                burst_len = 0;
            }
        }

        preds
    }
}

/// Configurable 1-second-window frequency-rule baseline.
#[derive(Debug, Clone)]
pub struct FrequencyRuleBaseline {
    pub band_center: f32,
    pub band_width: f32,
    /// Window length (ms).
    pub window_ms: usize,
    /// Minimum samples-in-band per window to call it "beat-bearing".
    pub min_in_band_per_window: usize,
}

impl Default for FrequencyRuleBaseline {
    fn default() -> Self {
        Self {
            band_center: 4500.0,
            band_width: 0.10,
            window_ms: 1000,
            min_in_band_per_window: 5,
        }
    }
}

impl FrequencyRuleBaseline {
    fn in_band(&self, freq: f32) -> bool {
        if freq <= 0.0 {
            return false;
        }
        let dev = (freq - self.band_center).abs() / self.band_center;
        dev <= self.band_width
    }

    /// Estimate beats per minute by counting QRS-like bursts inside
    /// each window and emit one classification at every window
    /// boundary. Bursts are tracked at sample level with a refractory
    /// period to avoid double-counting.
    pub fn run(&self, stream: &[f32]) -> Vec<BaselinePrediction> {
        let mut preds = Vec::new();
        let mut burst_count: usize = 0;
        let mut in_burst = false;
        let mut burst_start: usize = 0;
        let mut last_burst_end: usize = 0;
        let refractory: usize = 150;

        for (i, &freq) in stream.iter().enumerate() {
            let inside = self.in_band(freq);
            if inside {
                if !in_burst {
                    in_burst = true;
                    burst_start = i;
                }
            } else if in_burst {
                let burst_len = i.saturating_sub(burst_start);
                if burst_len >= 2 && burst_start.saturating_sub(last_burst_end) >= refractory {
                    burst_count += 1;
                    last_burst_end = i;
                }
                in_burst = false;
            }

            // emit one prediction per window boundary
            if (i + 1) % self.window_ms == 0 && burst_count >= 2 {
                let bpm = burst_count as f32 * 60.0 * 1000.0 / self.window_ms as f32;
                let class = if bpm > 100.0 {
                    RhythmClass::Tachycardia
                } else if bpm < 60.0 {
                    RhythmClass::Bradycardia
                } else {
                    RhythmClass::NormalSinus
                };
                preds.push(BaselinePrediction {
                    step: i,
                    class,
                    bpm,
                });
                burst_count = 0;
                last_burst_end = 0;
            } else if (i + 1) % self.window_ms == 0 {
                burst_count = 0;
                last_burst_end = 0;
            }
        }
        preds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::{generate, SyntheticConfig};

    #[test]
    fn threshold_burst_runs_on_clean_synth() {
        let cfg = SyntheticConfig::default()
            .with_seed(11)
            .with_beats_per_class(10)
            .with_irregular(false);
        let rec = generate(&cfg);
        let preds = ThresholdBurstBaseline::default().run(&rec.stream);
        assert!(
            !preds.is_empty(),
            "baseline must emit at least one prediction"
        );
    }

    #[test]
    fn frequency_rule_runs_on_clean_synth() {
        let cfg = SyntheticConfig::default()
            .with_seed(11)
            .with_beats_per_class(10)
            .with_irregular(false);
        let rec = generate(&cfg);
        let preds = FrequencyRuleBaseline::default().run(&rec.stream);
        // It is allowed to be empty if no window has ≥ 2 bursts, but
        // for 30 beats over a few seconds we expect output.
        assert!(
            !preds.is_empty(),
            "freq-rule baseline expected to emit on 30 beats"
        );
    }
}
