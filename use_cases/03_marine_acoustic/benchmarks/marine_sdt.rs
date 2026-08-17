// SPDX-License-Identifier: AGPL-3.0-only
//! SDT benchmark for the marine acoustic classifier.
//! Green & Swets (1966) — d', TPR, FPR with Wilson 95 % CI.
//! Date: 2026-04-24

use cricket_brain_marine::acoustic_signal;
use cricket_brain_marine::detector::{AcousticEvent, MarineDetector};

const TRIALS: usize = 200;
const SIGNAL_LEN: usize = 500;

fn wilson_ci(s: usize, n: usize) -> (f32, f32) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let (p, nf, z) = (s as f32 / n as f32, n as f32, 1.96f32);
    let z2 = z * z;
    let c = (p + z2 / (2.0 * nf)) / (1.0 + z2 / nf);
    let m = z * ((p * (1.0 - p) / nf + z2 / (4.0 * nf * nf)).sqrt()) / (1.0 + z2 / nf);
    ((c - m).max(0.0), (c + m).min(1.0))
}

fn inv_norm(p: f32) -> f32 {
    let p = p.clamp(0.0001, 0.9999);
    let t = if p < 0.5 {
        (-2.0 * p.ln()).sqrt()
    } else {
        (-2.0 * (1.0 - p).ln()).sqrt()
    };
    let v = t
        - (2.515517 + 0.802853 * t + 0.010328 * t * t)
            / (1.0 + 1.432788 * t + 0.189269 * t * t + 0.001308 * t * t * t);
    if p < 0.5 {
        -v
    } else {
        v
    }
}

fn d_prime(hr: f32, fa: f32) -> f32 {
    inv_norm(hr.clamp(0.001, 0.999)) - inv_norm(fa.clamp(0.001, 0.999))
}

fn run_condition(
    name: &str,
    target: AcousticEvent,
    target_signal: fn(usize) -> Vec<f32>,
    noise_signal: fn(usize) -> Vec<f32>,
) {
    let mut det = MarineDetector::new();
    let (mut hits, mut misses, mut fa, mut cr) = (0, 0, 0, 0);

    for _ in 0..TRIALS {
        det.reset();
        let sig = target_signal(SIGNAL_LEN);
        let mut last = None;
        for &f in &sig {
            if let Some(c) = det.step(f) {
                last = Some(c);
            }
        }
        if last == Some(target) {
            hits += 1;
        } else {
            misses += 1;
        }
    }

    for _ in 0..TRIALS {
        det.reset();
        let sig = noise_signal(SIGNAL_LEN);
        let mut last = None;
        for &f in &sig {
            if let Some(c) = det.step(f) {
                last = Some(c);
            }
        }
        if last == Some(target) {
            fa += 1;
        } else {
            cr += 1;
        }
    }

    let tpr = hits as f32 / (hits + misses).max(1) as f32;
    let fpr = fa as f32 / (fa + cr).max(1) as f32;
    let dp = d_prime(tpr, fpr);
    let (tpr_lo, tpr_hi) = wilson_ci(hits, hits + misses);
    let (fpr_lo, fpr_hi) = wilson_ci(fa, fa + cr);
    let rating = if dp > 3.0 {
        "EXCELLENT"
    } else if dp > 2.0 {
        "GOOD"
    } else if dp > 1.0 {
        "MODERATE"
    } else {
        "POOR"
    };

    println!("  {name:35} {tpr:.3}  {fpr:.3}  {dp:>6.2}  {rating}");
    println!("    TPR 95% CI: [{tpr_lo:.3}, {tpr_hi:.3}]  FPR 95% CI: [{fpr_lo:.3}, {fpr_hi:.3}]");
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Marine SDT Benchmark — Green & Swets (1966)               ║");
    println!("║  Date: 2026-04-24 | {TRIALS} trials/class | {SIGNAL_LEN} steps    ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!(
        "  {:<35} {:>5}  {:>5}  {:>6}  {}",
        "Condition", "TPR", "FPR", "d'", "Rating"
    );
    println!(
        "  {:─>35} {:─>5}  {:─>5}  {:─>6}  {:─>9}",
        "", "", "", "", ""
    );

    run_condition(
        "FinWhale vs Ambient",
        AcousticEvent::FinWhale,
        acoustic_signal::fin_whale_call,
        acoustic_signal::ambient_noise,
    );
    run_condition(
        "BlueWhale vs Ambient",
        AcousticEvent::BlueWhale,
        acoustic_signal::blue_whale_call,
        acoustic_signal::ambient_noise,
    );
    run_condition(
        "ShipNoise vs Ambient",
        AcousticEvent::ShipNoise,
        acoustic_signal::ship_passage,
        acoustic_signal::ambient_noise,
    );
    run_condition(
        "Humpback vs Ambient",
        AcousticEvent::Humpback,
        acoustic_signal::humpback_song,
        acoustic_signal::ambient_noise,
    );
    run_condition(
        "Ambient vs ShipNoise",
        AcousticEvent::Ambient,
        acoustic_signal::ambient_noise,
        acoustic_signal::ship_passage,
    );

    println!("\n  {TRIALS} signal + {TRIALS} noise trials per condition. d' > 3.0 = excellent.");
}
