//! # Adversarial Stress-Test Benchmark
//!
//! Addresses specific scientific critique points about the baseline comparison:
//!
//! 1. **Independent noise seeds** — 10 completely different RNG seeds per condition
//! 2. **Colored noise (pink 1/f)** — not just white/sparse frequency bursts
//! 3. **In-band interferer** — sustained 4400 Hz tone (inside Gaussian window)
//! 4. **Frequency deviation** — target at 4400, 4500, 4600 Hz
//! 5. **Extended silence** — 1000-step noise-only trials for FPR statistics
//! 6. **More trials** — 500 per condition for statistical power
//! 7. **Simple threshold baseline** — is the problem trivially solvable?
//! 8. **Additive white Gaussian noise** — proper AWGN, not just frequency bursts
//!
//! References:
//! - Green, D.M. & Swets, J.A. (1966). Signal Detection Theory.
//! - Wilson, E.B. (1927). Probable inference, JASA. (confidence intervals)

use cricket_brain::brain::{BrainConfig, CricketBrain};

// ---------------------------------------------------------------------------
// Deterministic RNG (LCG + Box-Muller for Gaussian)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / ((1u64 << 24) as f32)
    }

    /// Standard normal via Box-Muller transform
    fn gaussian(&mut self) -> f32 {
        let u1 = self.next_f32().max(1e-10);
        let u2 = self.next_f32();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos()
    }
}

// ---------------------------------------------------------------------------
// Wilson score 95% confidence interval
// ---------------------------------------------------------------------------

fn wilson_ci(successes: usize, total: usize) -> (f32, f32) {
    if total == 0 {
        return (0.0, 1.0);
    }
    let n = total as f32;
    let p = successes as f32 / n;
    let z = 1.96f32;
    let z2 = z * z;
    let center = (p + z2 / (2.0 * n)) / (1.0 + z2 / n);
    let margin = z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / (1.0 + z2 / n);
    ((center - margin).max(0.0), (center + margin).min(1.0))
}

// ---------------------------------------------------------------------------
// Noise models
// ---------------------------------------------------------------------------

/// Pure frequency-domain noise (original protocol): random bursts at random freqs
fn noise_freq_bursts(rng: &mut Rng, snr_db: i32) -> f32 {
    let noise_scale = 10.0_f32.powf(-snr_db as f32 / 20.0).min(4.0);
    let burst_prob = (0.03 * noise_scale).clamp(0.01, 0.18);
    if rng.next_f32() < burst_prob {
        2000.0 + rng.next_f32() * 6000.0
    } else {
        0.0
    }
}

/// AWGN: target freq + Gaussian frequency jitter
fn signal_with_awgn(rng: &mut Rng, target: f32, snr_db: i32) -> f32 {
    let noise_scale = 10.0_f32.powf(-snr_db as f32 / 20.0);
    let jitter = target * 0.05 * noise_scale * rng.gaussian();
    (target + jitter).clamp(1000.0, 9000.0)
}

/// Pink noise (1/f): accumulated random walk, mapped to frequency
fn noise_pink(rng: &mut Rng, pink_state: &mut f32, snr_db: i32) -> f32 {
    let noise_scale = 10.0_f32.powf(-snr_db as f32 / 20.0).min(4.0);
    // Random walk with mean-reversion
    *pink_state = *pink_state * 0.95 + rng.gaussian() * 500.0 * noise_scale;
    let freq = 4500.0 + *pink_state;
    if !(1000.0..=9000.0).contains(&freq) || rng.next_f32() > 0.3 {
        0.0 // Often silent
    } else {
        freq
    }
}

/// In-band interferer: sustained tone near target frequency
fn noise_inband_interferer(rng: &mut Rng, interferer_freq: f32, snr_db: i32) -> f32 {
    let noise_scale = 10.0_f32.powf(-snr_db as f32 / 20.0).min(4.0);
    // Interferer is always present with some probability
    let prob = (0.4 * noise_scale).clamp(0.1, 0.8);
    if rng.next_f32() < prob {
        interferer_freq + rng.gaussian() * 20.0
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------
// Trial runner
// ---------------------------------------------------------------------------

fn run_trial(
    brain: &mut CricketBrain,
    warmup: &[f32],
    observation: &[f32],
) -> (bool, Option<usize>) {
    brain.reset();
    for &f in warmup {
        let _ = brain.step(f);
    }
    let mut detected = false;
    let mut first = None;
    for (t, &f) in observation.iter().enumerate() {
        if brain.step(f) > 0.0 {
            detected = true;
            if first.is_none() {
                first = Some(t);
            }
        }
    }
    (detected, first)
}

// ---------------------------------------------------------------------------
// Test configurations
// ---------------------------------------------------------------------------

struct TestResult {
    name: &'static str,
    description: &'static str,
    tp: usize,
    fp: usize,
    n_signal: usize,
    n_noise: usize,
}

impl TestResult {
    fn tpr(&self) -> f32 {
        self.tp as f32 / self.n_signal.max(1) as f32
    }
    fn fpr(&self) -> f32 {
        self.fp as f32 / self.n_noise.max(1) as f32
    }
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Adversarial Stress-Test Benchmark                             ║");
    println!("║  Addressing scientific critique of baseline comparison          ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let mut brain = CricketBrain::new(BrainConfig::default()).expect("valid config");
    let mut results: Vec<TestResult> = Vec::new();

    // =====================================================================
    // TEST 1: Multiple independent seeds (original protocol)
    // =====================================================================
    {
        println!("─── Test 1: Independent Seeds (original noise model) ───\n");
        println!("  Running 10 completely different RNG seeds, 500 trials each, SNR=0 dB\n");

        let seeds: [u64; 10] = [
            1337,
            42,
            0xDEAD_BEEF,
            999_999_937,
            7919,
            0xCAFE_BABE,
            314159,
            271828,
            0xB0BA_F00D,
            65537,
        ];
        let mut total_tp = 0usize;
        let mut total_fp = 0usize;
        let mut total_signal = 0usize;
        let mut total_noise = 0usize;

        for &seed in &seeds {
            let mut rng = Rng::new(seed);
            let mut tp = 0;
            let mut fp = 0;
            let trials = 500;
            let snr = 0;

            for _ in 0..trials {
                // Signal-present
                let warmup: Vec<f32> = (0..24).map(|_| noise_freq_bursts(&mut rng, snr)).collect();
                let obs: Vec<f32> = (0..120)
                    .map(|t| {
                        if (32..92).contains(&t) {
                            signal_with_awgn(&mut rng, 4500.0, snr)
                        } else {
                            noise_freq_bursts(&mut rng, snr)
                        }
                    })
                    .collect();
                let (det, _) = run_trial(&mut brain, &warmup, &obs);
                if det {
                    tp += 1;
                }
            }

            for _ in 0..trials {
                // Noise-only
                let warmup: Vec<f32> = (0..24).map(|_| noise_freq_bursts(&mut rng, snr)).collect();
                let obs: Vec<f32> = (0..120).map(|_| noise_freq_bursts(&mut rng, snr)).collect();
                let (det, _) = run_trial(&mut brain, &warmup, &obs);
                if det {
                    fp += 1;
                }
            }

            let tpr = tp as f32 / trials as f32;
            let fpr = fp as f32 / trials as f32;
            println!(
                "  Seed {seed:>12}: TPR={tpr:.3} ({tp}/{trials})  FPR={fpr:.3} ({fp}/{trials})"
            );
            total_tp += tp;
            total_fp += fp;
            total_signal += trials;
            total_noise += trials;
        }

        let agg_tpr = total_tp as f32 / total_signal as f32;
        let agg_fpr = total_fp as f32 / total_noise as f32;
        let (tpr_lo, tpr_hi) = wilson_ci(total_tp, total_signal);
        let (fpr_lo, fpr_hi) = wilson_ci(total_fp, total_noise);
        println!("\n  Aggregate (5000+5000 trials):");
        println!("    TPR = {agg_tpr:.4} [{tpr_lo:.4}, {tpr_hi:.4}] 95% CI");
        println!("    FPR = {agg_fpr:.4} [{fpr_lo:.4}, {fpr_hi:.4}] 95% CI\n");

        results.push(TestResult {
            name: "Independent Seeds",
            description: "10 seeds, 500 trials each, SNR=0dB, original noise",
            tp: total_tp,
            fp: total_fp,
            n_signal: total_signal,
            n_noise: total_noise,
        });
    }

    // =====================================================================
    // TEST 2: AWGN (additive white Gaussian noise on frequency)
    // =====================================================================
    {
        println!("─── Test 2: AWGN (Gaussian frequency jitter) ───\n");
        let snr_levels = [-10, -5, 0, 5, 10, 20];
        let trials = 500;

        for &snr in &snr_levels {
            let mut rng = Rng::new(0xABCD_1234 ^ (snr as u64).wrapping_mul(0x9E37));
            let mut tp = 0;
            let mut fp = 0;

            for _ in 0..trials {
                let warmup: Vec<f32> = (0..24)
                    .map(|_| {
                        if rng.next_f32() < 0.05 {
                            2000.0 + rng.next_f32() * 6000.0
                        } else {
                            0.0
                        }
                    })
                    .collect();
                let obs: Vec<f32> = (0..120)
                    .map(|t| {
                        if (32..92).contains(&t) {
                            signal_with_awgn(&mut rng, 4500.0, snr)
                        } else if rng.next_f32() < 0.05 {
                            2000.0 + rng.next_f32() * 6000.0
                        } else {
                            0.0
                        }
                    })
                    .collect();
                if run_trial(&mut brain, &warmup, &obs).0 {
                    tp += 1;
                }
            }

            for _ in 0..trials {
                let warmup: Vec<f32> = (0..24)
                    .map(|_| {
                        if rng.next_f32() < 0.05 {
                            2000.0 + rng.next_f32() * 6000.0
                        } else {
                            0.0
                        }
                    })
                    .collect();
                let obs: Vec<f32> = (0..120)
                    .map(|_| {
                        if rng.next_f32() < 0.05 {
                            2000.0 + rng.next_f32() * 6000.0
                        } else {
                            0.0
                        }
                    })
                    .collect();
                if run_trial(&mut brain, &warmup, &obs).0 {
                    fp += 1;
                }
            }

            let tpr = tp as f32 / trials as f32;
            let fpr = fp as f32 / trials as f32;
            println!(
                "  SNR {snr:>3} dB: TPR={tpr:.3} ({tp}/{trials})  FPR={fpr:.3} ({fp}/{trials})"
            );
        }
        println!();
    }

    // =====================================================================
    // TEST 3: Pink noise (1/f colored noise)
    // =====================================================================
    {
        println!("─── Test 3: Pink Noise (1/f colored, spectrally structured) ───\n");
        let trials = 500;
        let snr = 0;
        let mut rng = Rng::new(0xA1CE_F00D_u64.wrapping_add(42));
        let mut tp = 0;
        let mut fp = 0;

        for _ in 0..trials {
            let mut pink_state = 0.0f32;
            let warmup: Vec<f32> = (0..24)
                .map(|_| noise_pink(&mut rng, &mut pink_state, snr))
                .collect();
            let obs: Vec<f32> = (0..120)
                .map(|t| {
                    if (32..92).contains(&t) {
                        signal_with_awgn(&mut rng, 4500.0, snr)
                    } else {
                        noise_pink(&mut rng, &mut pink_state, snr)
                    }
                })
                .collect();
            if run_trial(&mut brain, &warmup, &obs).0 {
                tp += 1;
            }
        }

        for _ in 0..trials {
            let mut pink_state = 0.0f32;
            let warmup: Vec<f32> = (0..24)
                .map(|_| noise_pink(&mut rng, &mut pink_state, snr))
                .collect();
            let obs: Vec<f32> = (0..120)
                .map(|_| noise_pink(&mut rng, &mut pink_state, snr))
                .collect();
            if run_trial(&mut brain, &warmup, &obs).0 {
                fp += 1;
            }
        }

        let tpr = tp as f32 / trials as f32;
        let fpr = fp as f32 / trials as f32;
        let (fpr_lo, fpr_hi) = wilson_ci(fp, trials);
        println!("  TPR = {tpr:.3} ({tp}/{trials})");
        println!("  FPR = {fpr:.3} ({fp}/{trials}) [{fpr_lo:.4}, {fpr_hi:.4}] 95% CI\n");

        results.push(TestResult {
            name: "Pink Noise (1/f)",
            description: "Colored noise, SNR=0dB, 500 trials",
            tp,
            fp,
            n_signal: trials,
            n_noise: trials,
        });
    }

    // =====================================================================
    // TEST 4: In-band interferer (4400 Hz — inside Gaussian window)
    // =====================================================================
    {
        println!("─── Test 4: In-Band Interferer (4400 Hz, inside ±10% window) ───\n");
        let interferer_freqs: [(f32, &str); 4] = [
            (4400.0, "4400 Hz (-2.2%)"),
            (4300.0, "4300 Hz (-4.4%)"),
            (4050.0, "4050 Hz (-10%)"),
            (4000.0, "4000 Hz (-11.1%)"),
        ];
        let trials = 500;

        for (freq, label) in &interferer_freqs {
            let mut rng = Rng::new(0xABCD_0001_u64.wrapping_add(*freq as u64));
            let mut fp = 0;

            // Only noise trials — interferer pretending to be target
            for _ in 0..trials {
                let warmup: Vec<f32> = (0..24)
                    .map(|_| noise_inband_interferer(&mut rng, *freq, 0))
                    .collect();
                let obs: Vec<f32> = (0..120)
                    .map(|_| noise_inband_interferer(&mut rng, *freq, 0))
                    .collect();
                if run_trial(&mut brain, &warmup, &obs).0 {
                    fp += 1;
                }
            }

            let fpr = fp as f32 / trials as f32;
            let (lo, hi) = wilson_ci(fp, trials);
            println!("  {label}: FPR = {fpr:.3} ({fp}/{trials}) [{lo:.4}, {hi:.4}] 95% CI");
        }
        println!();
    }

    // =====================================================================
    // TEST 5: Frequency deviation (detect off-center targets)
    // =====================================================================
    {
        println!("─── Test 5: Frequency Deviation (target not at 4500 Hz) ───\n");
        let target_freqs: [(f32, &str); 5] = [
            (4500.0, "4500 Hz (exact)"),
            (4400.0, "4400 Hz (-2.2%)"),
            (4600.0, "4600 Hz (+2.2%)"),
            (4050.0, "4050 Hz (-10%, boundary)"),
            (4950.0, "4950 Hz (+10%, boundary)"),
        ];
        let trials = 500;

        for (freq, label) in &target_freqs {
            let mut tp = 0;

            for _ in 0..trials {
                let warmup: Vec<f32> = (0..24).map(|_| 0.0f32).collect();
                let obs: Vec<f32> = (0..120)
                    .map(|t| if (32..92).contains(&t) { *freq } else { 0.0 })
                    .collect();
                if run_trial(&mut brain, &warmup, &obs).0 {
                    tp += 1;
                }
            }

            let tpr = tp as f32 / trials as f32;
            println!("  {label}: TPR = {tpr:.3} ({tp}/{trials})");
        }
        println!();
    }

    // =====================================================================
    // TEST 6: Extended silence (1000-step trials for FPR power)
    // =====================================================================
    {
        println!("─── Test 6: Extended Silence (1000 steps, 2000 trials) ───\n");
        let mut rng = Rng::new(0xAA00_BB11_u64.wrapping_add(7));
        let trials = 2000;
        let mut fp = 0;

        for _ in 0..trials {
            brain.reset();
            let mut detected = false;
            for _ in 0..1000 {
                if brain.step(0.0) > 0.0 {
                    detected = true;
                    break;
                }
            }
            if detected {
                fp += 1;
            }
        }

        let fpr = fp as f32 / trials as f32;
        let (lo, hi) = wilson_ci(fp, trials);
        println!("  Pure silence (0 Hz, 1000 steps): FPR = {fpr:.4} ({fp}/{trials}) [{lo:.4}, {hi:.4}] 95% CI");

        // Also test with random background noise
        let mut fp_noisy = 0;
        for _ in 0..trials {
            brain.reset();
            let mut detected = false;
            for _ in 0..1000 {
                let freq = if rng.next_f32() < 0.1 {
                    2000.0 + rng.next_f32() * 6000.0
                } else {
                    0.0
                };
                if brain.step(freq) > 0.0 {
                    detected = true;
                    break;
                }
            }
            if detected {
                fp_noisy += 1;
            }
        }

        let fpr_noisy = fp_noisy as f32 / trials as f32;
        let (lo2, hi2) = wilson_ci(fp_noisy, trials);
        println!("  Random bursts (10% density, 1000 steps): FPR = {fpr_noisy:.4} ({fp_noisy}/{trials}) [{lo2:.4}, {hi2:.4}] 95% CI\n");

        results.push(TestResult {
            name: "Extended Silence",
            description: "1000-step trials, 2000 trials, pure silence",
            tp: 0,
            fp,
            n_signal: 0,
            n_noise: trials,
        });
    }

    // =====================================================================
    // TEST 7: Simple energy threshold detector comparison
    // =====================================================================
    {
        println!("─── Test 7: Simple Threshold Detector (is the problem trivial?) ───\n");
        println!("  Testing if a simple 'any non-zero input in window' detector works.\n");

        let trials = 500;
        let snr_levels = [-10, 0, 10, 20];

        for &snr in &snr_levels {
            let mut rng = Rng::new(0xCC00_DD11_u64.wrapping_add(snr as u64));
            let mut tp_simple = 0;
            let mut fp_simple = 0;
            let mut tp_brain = 0;
            let mut fp_brain = 0;

            for _ in 0..trials {
                // Signal present
                let warmup: Vec<f32> = (0..24).map(|_| noise_freq_bursts(&mut rng, snr)).collect();
                let obs: Vec<f32> = (0..120)
                    .map(|t| {
                        if (32..92).contains(&t) {
                            signal_with_awgn(&mut rng, 4500.0, snr)
                        } else {
                            noise_freq_bursts(&mut rng, snr)
                        }
                    })
                    .collect();

                // Simple: any non-zero freq in observation window = detected
                let simple_det = obs.iter().any(|&f| f > 0.0);
                if simple_det {
                    tp_simple += 1;
                }
                if run_trial(&mut brain, &warmup, &obs).0 {
                    tp_brain += 1;
                }
            }

            for _ in 0..trials {
                // Noise only
                let warmup: Vec<f32> = (0..24).map(|_| noise_freq_bursts(&mut rng, snr)).collect();
                let obs: Vec<f32> = (0..120).map(|_| noise_freq_bursts(&mut rng, snr)).collect();

                let simple_det = obs.iter().any(|&f| f > 0.0);
                if simple_det {
                    fp_simple += 1;
                }
                if run_trial(&mut brain, &warmup, &obs).0 {
                    fp_brain += 1;
                }
            }

            println!("  SNR {snr:>3} dB:");
            println!(
                "    Simple threshold: TPR={:.3} FPR={:.3}",
                tp_simple as f32 / trials as f32,
                fp_simple as f32 / trials as f32
            );
            println!(
                "    CricketBrain:     TPR={:.3} FPR={:.3}",
                tp_brain as f32 / trials as f32,
                fp_brain as f32 / trials as f32
            );
        }
        println!();
    }

    // =====================================================================
    // SUMMARY
    // =====================================================================
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Stress-Test Summary                                           ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    for r in &results {
        println!(
            "║  {:25} TPR={:.3}  FPR={:.3}  ({})  ║",
            r.name,
            r.tpr(),
            r.fpr(),
            r.description
        );
    }
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  KNOWN LIMITATIONS (honest disclosure):                        ║");
    println!("║  • Gaussian tuning rejects off-frequency noise trivially       ║");
    println!("║  • In-band interferers within ±10% CAN cause false positives   ║");
    println!("║  • Frequency resolution ~100 Hz (not 9 Hz like human hearing)  ║");
    println!("║  • All noise is frequency-domain (no time-domain amplitude)    ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}
