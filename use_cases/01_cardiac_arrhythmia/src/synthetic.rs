// SPDX-License-Identifier: AGPL-3.0-only
//! Synthetic ECG dataset generator with **explicit ground-truth labels**
//! and seedable, parameterised variability.
//!
//! Goals:
//!
//! - Every generated cycle carries a [`RhythmClass`] ground-truth label so
//!   that the benchmark never has to infer truth from the prediction.
//! - All variability is driven by an explicit `u64` seed (deterministic
//!   reproducibility).
//! - Stress transforms (noise, baseline wander, amplitude scaling, HR
//!   variability, morphology jitter, motion-artifact bursts, weak peaks)
//!   are first-class operations — not buried inside benchmark binaries.
//!
//! The output is a flat `Vec<f32>` frequency stream and a parallel
//! `Vec<LabeledSegment>` describing which class produced which step
//! range. Predictions emitted by `CardiacDetector` carry the timestep
//! at which the classification occurred, so we can map prediction →
//! label by looking up the segment that contained that step.

use crate::detector::RhythmClass;

/// Frequencies and durations of one cardiac cycle.
#[derive(Debug, Clone, Copy)]
pub struct CycleParams {
    pub p_freq_hz: f32,
    pub p_dur_ms: u32,
    pub gap1_ms: u32,
    pub qrs_freq_hz: f32,
    pub qrs_dur_ms: u32,
    pub gap2_ms: u32,
    pub t_freq_hz: f32,
    pub t_dur_ms: u32,
}

impl Default for CycleParams {
    fn default() -> Self {
        Self {
            p_freq_hz: 3100.0,
            p_dur_ms: 12,
            gap1_ms: 4,
            qrs_freq_hz: 4500.0,
            qrs_dur_ms: 10,
            gap2_ms: 4,
            t_freq_hz: 3400.0,
            t_dur_ms: 14,
        }
    }
}

impl CycleParams {
    /// Total wave-active duration (P + gap + QRS + gap + T).
    pub fn wave_dur_ms(&self) -> u32 {
        self.p_dur_ms + self.gap1_ms + self.qrs_dur_ms + self.gap2_ms + self.t_dur_ms
    }
}

/// One ground-truth-labelled segment of the synthetic stream.
#[derive(Debug, Clone)]
pub struct LabeledSegment {
    pub class: RhythmClass,
    pub start_step: usize,
    pub end_step: usize,
    /// Mean RR interval used to generate this segment (ms).
    pub mean_rr_ms: u32,
}

impl LabeledSegment {
    pub fn contains(&self, step: usize) -> bool {
        step >= self.start_step && step < self.end_step
    }
}

/// Configuration for one synthetic recording.
#[derive(Debug, Clone)]
pub struct SyntheticConfig {
    /// Master seed — every random draw is derived from this seed.
    pub seed: u64,
    /// Number of beats per ground-truth class block.
    pub beats_per_class: u32,
    /// Whether to include the Irregular ground-truth class
    /// (random RR ∈ `[300, 1200] ms`).
    pub include_irregular: bool,
    /// Heart-rate variability (HRV) — fractional jitter applied to the
    /// per-beat RR interval. `0.0` = perfectly regular.
    pub hrv: f32,
    /// Probability per millisecond of injecting a single in-band noise
    /// spike (in [0, 1]). `0.0` = clean.
    pub noise_prob: f32,
    /// Amplitude/frequency scaling applied to the QRS frequency: each
    /// beat shifts the QRS frequency by a random factor in
    /// `[1 - amp_jitter, 1 + amp_jitter]`. `0.0` = no jitter.
    pub amp_jitter: f32,
    /// Baseline-wander amplitude: a slow sinusoid added to all
    /// frequency samples (Hz peak-to-peak). `0.0` = none.
    pub baseline_wander_hz: f32,
    /// Baseline-wander rate (Hz of the modulating sinusoid). Typical
    /// breathing artifact: 0.2 – 0.4 Hz.
    pub baseline_wander_rate_hz: f32,
    /// Morphology jitter: each per-cycle parameter (p/qrs/t freq and
    /// duration) is independently scaled by `1 ± morph_jitter`.
    pub morph_jitter: f32,
    /// Probability of dropping a QRS burst entirely (missing/weak peak).
    pub missing_qrs_prob: f32,
    /// Probability per millisecond of a motion-artifact burst (a
    /// short broadband spike) starting.
    pub motion_burst_prob: f32,
    /// Mean duration of a motion-artifact burst (ms).
    pub motion_burst_ms: u32,
}

impl Default for SyntheticConfig {
    fn default() -> Self {
        Self {
            seed: 0xCAFE_F00D,
            beats_per_class: 30,
            include_irregular: true,
            hrv: 0.0,
            noise_prob: 0.0,
            amp_jitter: 0.0,
            baseline_wander_hz: 0.0,
            baseline_wander_rate_hz: 0.25,
            morph_jitter: 0.0,
            missing_qrs_prob: 0.0,
            motion_burst_prob: 0.0,
            motion_burst_ms: 8,
        }
    }
}

impl SyntheticConfig {
    /// Builder helper.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
    pub fn with_beats_per_class(mut self, n: u32) -> Self {
        self.beats_per_class = n;
        self
    }
    pub fn with_hrv(mut self, hrv: f32) -> Self {
        self.hrv = hrv;
        self
    }
    pub fn with_noise(mut self, p: f32) -> Self {
        self.noise_prob = p;
        self
    }
    pub fn with_amp_jitter(mut self, a: f32) -> Self {
        self.amp_jitter = a;
        self
    }
    pub fn with_baseline_wander(mut self, hz: f32) -> Self {
        self.baseline_wander_hz = hz;
        self
    }
    pub fn with_morph_jitter(mut self, m: f32) -> Self {
        self.morph_jitter = m;
        self
    }
    pub fn with_missing_qrs(mut self, p: f32) -> Self {
        self.missing_qrs_prob = p;
        self
    }
    pub fn with_motion_burst(mut self, p: f32) -> Self {
        self.motion_burst_prob = p;
        self
    }
    pub fn with_irregular(mut self, on: bool) -> Self {
        self.include_irregular = on;
        self
    }
}

/// Tiny deterministic SplitMix64-style RNG. The whole synthetic
/// pipeline runs on this — no external crates.
#[derive(Debug, Clone)]
pub struct DetRng {
    state: u64,
}

impl DetRng {
    pub fn new(seed: u64) -> Self {
        // SplitMix64 avalanche on the input seed so 0 is also fine.
        let mut x = seed.wrapping_add(0x9E3779B97F4A7C15);
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
        x ^= x >> 31;
        Self { state: x.max(1) }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    /// Uniform `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Symmetric `[-1, 1]`.
    pub fn sym(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
    /// Uniform integer in `[lo, hi]`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        let span = hi.saturating_sub(lo).saturating_add(1).max(1);
        lo + (self.next_u64() as u32) % span
    }
}

/// Convenience: classify a ground-truth RR interval into one of the
/// rate-regime classes used by [`CardiacDetector`]. Irregular is
/// treated separately via the `include_irregular` knob.
pub fn ground_truth_class(rr_ms: u32) -> RhythmClass {
    let bpm = 60_000.0 / rr_ms as f32;
    if bpm > 100.0 {
        RhythmClass::Tachycardia
    } else if bpm < 60.0 {
        RhythmClass::Bradycardia
    } else {
        RhythmClass::NormalSinus
    }
}

/// Generate one cardiac cycle as a frequency-stream segment, with
/// per-beat morphology jitter and amplitude jitter. The QRS section
/// can optionally be dropped entirely (missing_qrs).
fn render_cycle(
    rng: &mut DetRng,
    base: &CycleParams,
    cfg: &SyntheticConfig,
    drop_qrs: bool,
) -> Vec<f32> {
    // Per-beat morphology jitter
    let mj = cfg.morph_jitter.clamp(0.0, 0.95);
    let aj = cfg.amp_jitter.clamp(0.0, 0.95);

    let scale = |v: f32, jitter: f32, rng: &mut DetRng| -> f32 {
        if jitter == 0.0 {
            v
        } else {
            v * (1.0 + jitter * rng.sym())
        }
    };

    let p_freq = scale(base.p_freq_hz, mj, rng);
    let p_dur = (base.p_dur_ms as f32 * (1.0 + mj * rng.sym())).max(1.0) as u32;
    let qrs_freq_jit = scale(base.qrs_freq_hz, mj.max(aj), rng);
    let qrs_dur = (base.qrs_dur_ms as f32 * (1.0 + mj * rng.sym())).max(1.0) as u32;
    let t_freq = scale(base.t_freq_hz, mj, rng);
    let t_dur = (base.t_dur_ms as f32 * (1.0 + mj * rng.sym())).max(1.0) as u32;

    let mut out =
        Vec::with_capacity((p_dur + base.gap1_ms + qrs_dur + base.gap2_ms + t_dur) as usize);
    out.extend(std::iter::repeat(p_freq).take(p_dur as usize));
    out.extend(std::iter::repeat(0.0).take(base.gap1_ms as usize));
    if drop_qrs {
        // Replace QRS with silence so the rest of the cycle still has the
        // expected shape but no detectable beat lands inside.
        out.extend(std::iter::repeat(0.0).take(qrs_dur as usize));
    } else {
        out.extend(std::iter::repeat(qrs_freq_jit).take(qrs_dur as usize));
    }
    out.extend(std::iter::repeat(0.0).take(base.gap2_ms as usize));
    out.extend(std::iter::repeat(t_freq).take(t_dur as usize));
    out
}

/// One labelled synthetic recording with explicit ground-truth.
#[derive(Debug, Clone)]
pub struct SyntheticRecording {
    pub stream: Vec<f32>,
    pub segments: Vec<LabeledSegment>,
    pub config: SyntheticConfig,
}

impl SyntheticRecording {
    /// Look up the ground-truth label for a given step. Returns `None`
    /// if the step is outside any labelled segment.
    pub fn label_for_step(&self, step: usize) -> Option<RhythmClass> {
        // Linear scan — number of segments is small (≤ a few hundred).
        for s in &self.segments {
            if s.contains(step) {
                return Some(s.class);
            }
        }
        None
    }
}

/// Targets used to split the dataset into class blocks.
fn rr_for_class(class: RhythmClass, rng: &mut DetRng) -> u32 {
    match class {
        RhythmClass::NormalSinus => rng.range(700, 1000), // 60 – 86 BPM
        RhythmClass::Tachycardia => rng.range(330, 500),  // 120 – 182 BPM
        RhythmClass::Bradycardia => rng.range(1200, 1800), // 33 – 50 BPM
        RhythmClass::Irregular => rng.range(300, 1200),   // chaotic
    }
}

/// Build a labelled synthetic recording according to `cfg`.
///
/// The recording layout is:
/// ```text
///   [ Normal block ][ Tachy block ][ Brady block ][ Irregular block ]
/// ```
/// `Irregular` is omitted if `cfg.include_irregular = false`.
pub fn generate(cfg: &SyntheticConfig) -> SyntheticRecording {
    let base = CycleParams::default();
    let mut rng = DetRng::new(cfg.seed);

    let classes: &[RhythmClass] = if cfg.include_irregular {
        &[
            RhythmClass::NormalSinus,
            RhythmClass::Tachycardia,
            RhythmClass::Bradycardia,
            RhythmClass::Irregular,
        ]
    } else {
        &[
            RhythmClass::NormalSinus,
            RhythmClass::Tachycardia,
            RhythmClass::Bradycardia,
        ]
    };

    let mut stream: Vec<f32> =
        Vec::with_capacity(cfg.beats_per_class as usize * classes.len() * 800);
    let mut segments = Vec::with_capacity(classes.len());

    for &class in classes {
        let block_start = stream.len();
        let mut sum_rr: u64 = 0;
        let mut n_rr: u64 = 0;

        for _ in 0..cfg.beats_per_class {
            // RR for this beat
            let mut rr_target = rr_for_class(class, &mut rng) as f32;
            if cfg.hrv > 0.0 {
                rr_target *= 1.0 + cfg.hrv * rng.sym();
            }
            let rr = rr_target.max(160.0) as u32;
            sum_rr += rr as u64;
            n_rr += 1;

            // Random beat-level perturbations
            let drop_qrs = cfg.missing_qrs_prob > 0.0 && rng.unit() < cfg.missing_qrs_prob;
            let mut beat = render_cycle(&mut rng, &base, cfg, drop_qrs);

            // Pad with silence to reach the target RR.
            let wave_len = beat.len() as u32;
            if rr > wave_len {
                beat.extend(std::iter::repeat(0.0).take((rr - wave_len) as usize));
            }
            stream.extend_from_slice(&beat);
        }

        let mean_rr = if n_rr > 0 { (sum_rr / n_rr) as u32 } else { 0 };
        segments.push(LabeledSegment {
            class,
            start_step: block_start,
            end_step: stream.len(),
            mean_rr_ms: mean_rr,
        });
    }

    // ---------------- Whole-stream perturbations ----------------

    // Baseline wander = slow sinusoid added to *every* sample.
    if cfg.baseline_wander_hz > 0.0 {
        let f = cfg.baseline_wander_rate_hz.max(0.01);
        let two_pi = std::f32::consts::TAU;
        for (i, s) in stream.iter_mut().enumerate() {
            let phase = two_pi * f * (i as f32) / 1000.0;
            *s += cfg.baseline_wander_hz * phase.sin();
            // Clip to non-negative — frequency stream cannot go below 0.
            if *s < 0.0 {
                *s = 0.0;
            }
        }
    }

    // Random in-band noise spikes.
    if cfg.noise_prob > 0.0 {
        for s in stream.iter_mut() {
            if rng.unit() < cfg.noise_prob {
                // Inject anywhere in the wide audible band CricketBrain sees.
                *s = 1000.0 + rng.unit() * 8000.0;
            }
        }
    }

    // Motion-artifact bursts (broadband, several samples long).
    if cfg.motion_burst_prob > 0.0 {
        let burst_len = cfg.motion_burst_ms.max(1) as usize;
        let n = stream.len();
        let mut i = 0;
        while i < n {
            if rng.unit() < cfg.motion_burst_prob {
                let end = (i + burst_len).min(n);
                for s in &mut stream[i..end] {
                    *s = 2000.0 + rng.unit() * 7000.0;
                }
                i = end;
            } else {
                i += 1;
            }
        }
    }

    SyntheticRecording {
        stream,
        segments,
        config: cfg.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_seed() {
        let cfg = SyntheticConfig::default()
            .with_seed(42)
            .with_beats_per_class(8)
            .with_irregular(false)
            .with_hrv(0.05);
        let a = generate(&cfg);
        let b = generate(&cfg);
        assert_eq!(a.stream.len(), b.stream.len());
        assert_eq!(a.stream, b.stream);
        assert_eq!(a.segments.len(), b.segments.len());
    }

    #[test]
    fn segments_cover_stream_contiguously() {
        let cfg = SyntheticConfig::default()
            .with_seed(1)
            .with_beats_per_class(5)
            .with_irregular(true);
        let rec = generate(&cfg);
        assert_eq!(rec.segments.first().unwrap().start_step, 0);
        for w in rec.segments.windows(2) {
            assert_eq!(
                w[0].end_step, w[1].start_step,
                "segments must be contiguous"
            );
        }
        assert_eq!(rec.segments.last().unwrap().end_step, rec.stream.len());
    }

    #[test]
    fn ground_truth_class_buckets() {
        assert_eq!(ground_truth_class(800), RhythmClass::NormalSinus);
        assert_eq!(ground_truth_class(400), RhythmClass::Tachycardia);
        assert_eq!(ground_truth_class(1500), RhythmClass::Bradycardia);
    }

    #[test]
    fn baseline_wander_changes_silent_samples() {
        let cfg = SyntheticConfig::default()
            .with_seed(7)
            .with_beats_per_class(4)
            .with_irregular(false)
            .with_baseline_wander(50.0);
        let rec = generate(&cfg);
        // At least *some* silent samples should now be non-zero.
        let lifted = rec.stream.iter().filter(|&&s| s > 0.0 && s < 200.0).count();
        assert!(
            lifted > 0,
            "baseline wander should lift some silent samples"
        );
    }
}
