//! # Signal Detection Theory (SDT) Benchmark
//!
//! Standard psychophysics methodology for evaluating detector performance.
//! Used universally in auditory neuroscience (Green & Swets, 1966).
//!
//! ## Metrics
//! - **d' (d-prime)**: Sensitivity index = z(Hit Rate) - z(False Alarm Rate)
//!   - d' < 1.0: Poor detection
//!   - d' 1.0–2.0: Moderate
//!   - d' 2.0–3.0: Good
//!   - d' > 3.0: Excellent (near ceiling)
//!
//! - **ROC Curve**: True Positive Rate vs False Positive Rate at multiple thresholds
//! - **AUC**: Area Under ROC Curve (1.0 = perfect, 0.5 = chance)
//!
//! ## Method
//! Present signal-present and signal-absent trials in randomized order.
//! The system must classify each trial as "detected" or "not detected".
//!
//! Reference: Green, D.M. & Swets, J.A. (1966). Signal Detection Theory
//! and Psychophysics. New York: Wiley.

use cricket_brain::brain::CricketBrain;

const TRIAL_DURATION_MS: usize = 100;
const N_SIGNAL_TRIALS: usize = 500;
const N_NOISE_TRIALS: usize = 500;
const SIGNAL_FREQ: f32 = 4500.0;
const RAMP_UP_MS: usize = 20; // discard initial transient

/// Inverse normal CDF approximation (Abramowitz & Stegun, 1964, formula 26.2.23).
/// Used to convert hit/FA rates to z-scores for d-prime calculation.
fn z_score(p: f64) -> f64 {
    if p <= 0.0001 {
        return -3.719;
    }
    if p >= 0.9999 {
        return 3.719;
    }

    let p_adj = if p > 0.5 { 1.0 - p } else { p };
    let t = (-2.0 * p_adj.ln()).sqrt();

    // Rational approximation coefficients
    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;

    let z = t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t);
    if p > 0.5 {
        z
    } else {
        -z
    }
}

/// Runs a single trial: feeds `freq` for `TRIAL_DURATION_MS` steps,
/// returns the peak ON1 output (after ramp-up discard).
fn run_trial(brain: &mut CricketBrain, freq: f32) -> f32 {
    brain.reset();
    let mut peak = 0.0_f32;
    for step in 0..TRIAL_DURATION_MS {
        let out = brain.step(freq);
        if step >= RAMP_UP_MS {
            peak = peak.max(out);
        }
    }
    peak
}

/// Compute ROC curve: sweep threshold from 0.0 to 1.0,
/// calculate (FPR, TPR) at each point.
fn compute_roc(signal_outputs: &[f32], noise_outputs: &[f32], n_points: usize) -> Vec<(f64, f64)> {
    let mut roc = Vec::with_capacity(n_points + 2);
    roc.push((1.0, 1.0)); // threshold = 0

    for i in 1..=n_points {
        let threshold = i as f32 / n_points as f32;
        let tpr = signal_outputs.iter().filter(|&&v| v >= threshold).count() as f64
            / signal_outputs.len() as f64;
        let fpr = noise_outputs.iter().filter(|&&v| v >= threshold).count() as f64
            / noise_outputs.len() as f64;
        roc.push((fpr, tpr));
    }

    roc.push((0.0, 0.0)); // threshold = 1.0+
                          // Sort by FPR ascending, then TPR DESCENDING so dedup keeps the
                          // highest TPR at each FPR level (dedup retains the first of equals).
    roc.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap()
            .then(b.1.partial_cmp(&a.1).unwrap()) // TPR descending
    });
    roc.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9);
    roc
}

/// Trapezoidal AUC from sorted ROC points.
fn compute_auc(roc: &[(f64, f64)]) -> f64 {
    let mut auc = 0.0;
    for i in 1..roc.len() {
        let dx = roc[i].0 - roc[i - 1].0;
        let avg_y = (roc[i].1 + roc[i - 1].1) / 2.0;
        auc += dx * avg_y;
    }
    auc.abs()
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Signal Detection Theory (SDT) Benchmark                   ║");
    println!("║  Green & Swets (1966) methodology                          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut brain = CricketBrain::new(Default::default()).unwrap();

    // === Condition 1: Target frequency (4500 Hz) vs Silence ===
    println!("─── Condition 1: Target (4500 Hz) vs Silence ───\n");

    let mut signal_outputs = Vec::with_capacity(N_SIGNAL_TRIALS);
    let mut noise_outputs = Vec::with_capacity(N_NOISE_TRIALS);

    for _ in 0..N_SIGNAL_TRIALS {
        signal_outputs.push(run_trial(&mut brain, SIGNAL_FREQ));
    }
    for _ in 0..N_NOISE_TRIALS {
        noise_outputs.push(run_trial(&mut brain, 0.0));
    }

    let threshold = 0.01; // any output > 0 counts as detection
    let hits = signal_outputs.iter().filter(|&&v| v > threshold).count();
    let false_alarms = noise_outputs.iter().filter(|&&v| v > threshold).count();
    let hit_rate = hits as f64 / N_SIGNAL_TRIALS as f64;
    let fa_rate = false_alarms as f64 / N_NOISE_TRIALS as f64;
    let d_prime = z_score(hit_rate) - z_score(fa_rate);
    let criterion_c = -0.5 * (z_score(hit_rate) + z_score(fa_rate));

    let roc = compute_roc(&signal_outputs, &noise_outputs, 100);
    let auc = compute_auc(&roc);

    println!("  Signal trials:    {N_SIGNAL_TRIALS}");
    println!("  Noise trials:     {N_NOISE_TRIALS}");
    println!("  Trial duration:   {TRIAL_DURATION_MS} ms");
    println!("  Hit Rate:         {hit_rate:.4} ({hits}/{N_SIGNAL_TRIALS})");
    println!("  False Alarm Rate: {fa_rate:.4} ({false_alarms}/{N_NOISE_TRIALS})");
    println!("  d' (sensitivity): {d_prime:.3}");
    println!("  c  (criterion):   {criterion_c:.3}");
    println!("  AUC (ROC):        {auc:.4}");
    print_dprime_rating(d_prime);

    // === Condition 2: Target vs Non-target frequency (2000 Hz) ===
    println!("\n─── Condition 2: Target (4500 Hz) vs Non-target (2000 Hz) ───\n");

    let mut nontarget_outputs = Vec::with_capacity(N_NOISE_TRIALS);
    for _ in 0..N_NOISE_TRIALS {
        nontarget_outputs.push(run_trial(&mut brain, 2000.0));
    }

    let fa2 = nontarget_outputs.iter().filter(|&&v| v > threshold).count();
    let fa_rate2 = fa2 as f64 / N_NOISE_TRIALS as f64;
    let d_prime2 = z_score(hit_rate) - z_score(fa_rate2);
    let roc2 = compute_roc(&signal_outputs, &nontarget_outputs, 100);
    let auc2 = compute_auc(&roc2);

    println!("  Hit Rate:         {hit_rate:.4}");
    println!("  FA Rate (2kHz):   {fa_rate2:.4} ({fa2}/{N_NOISE_TRIALS})");
    println!("  d':               {d_prime2:.3}");
    println!("  AUC:              {auc2:.4}");
    print_dprime_rating(d_prime2);

    // === Condition 3: Near-boundary frequency (4050 Hz, ~10% deviation) ===
    println!("\n─── Condition 3: Target (4500 Hz) vs Boundary (4050 Hz, -10%) ───\n");

    let mut boundary_outputs = Vec::with_capacity(N_NOISE_TRIALS);
    for _ in 0..N_NOISE_TRIALS {
        boundary_outputs.push(run_trial(&mut brain, 4050.0));
    }

    let fa3 = boundary_outputs.iter().filter(|&&v| v > threshold).count();
    let fa_rate3 = fa3 as f64 / N_NOISE_TRIALS as f64;
    let d_prime3 = z_score(hit_rate) - z_score(fa_rate3);
    let roc3 = compute_roc(&signal_outputs, &boundary_outputs, 100);
    let auc3 = compute_auc(&roc3);

    println!("  Hit Rate:         {hit_rate:.4}");
    println!("  FA Rate (4050Hz): {fa_rate3:.4} ({fa3}/{N_NOISE_TRIALS})");
    println!("  d':               {d_prime3:.3}");
    println!("  AUC:              {auc3:.4}");
    print_dprime_rating(d_prime3);

    // === ROC Sample Points ===
    println!("\n─── ROC Curve (Condition 1, sampled) ───\n");
    println!("  {:>8} {:>8}", "FPR", "TPR");
    println!("  {:>8} {:>8}", "────", "────");
    let sample_indices = [0, 5, 10, 20, 40, 60, 80, 95, 100];
    for &i in &sample_indices {
        if i < roc.len() {
            println!("  {:>8.4} {:>8.4}", roc[i].0, roc[i].1);
        }
    }

    // === Summary ===
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  SDT Summary                                               ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Cond 1 (Target vs Silence):     d'={d_prime:>6.3}  AUC={auc:.4}  ║");
    println!("║  Cond 2 (Target vs 2kHz):        d'={d_prime2:>6.3}  AUC={auc2:.4}  ║");
    println!("║  Cond 3 (Target vs Boundary):    d'={d_prime3:>6.3}  AUC={auc3:.4}  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}

fn print_dprime_rating(d: f64) {
    let rating = if d > 3.5 {
        "EXCELLENT (near ceiling)"
    } else if d > 2.0 {
        "GOOD"
    } else if d > 1.0 {
        "MODERATE"
    } else {
        "POOR"
    };
    println!("  Rating:           {rating}");
}
