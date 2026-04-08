//! # Temporal Gap Detection Benchmark
//!
//! Standard auditory neuroscience paradigm measuring minimum detectable
//! silent gap within a continuous tone (Plomp, 1964; Fitzgibbons & Wightman, 1982).
//!
//! ## Method
//! Present a continuous tone at carrier frequency, insert a silent gap of
//! varying duration, and measure whether the system's output reflects the gap.
//! A gap is "detected" if ON1 output drops to 0 during the gap and
//! recovers after the gap.
//!
//! ## Metrics
//! - **Minimum detectable gap (MDG)**: Shortest gap that causes output drop
//! - **Recovery time**: Timesteps from gap end to first post-gap spike
//! - **Gap transfer function**: Detection rate vs gap duration
//!
//! ## Reference Values
//! - Human MDG at 4 kHz: ~2-3 ms (Plomp, 1964)
//! - Cricket: ~5 ms inter-pulse interval selectivity (Pollack, 1998)
//!
//! ## References
//! - Plomp, R. (1964). Rate of decay of auditory sensation. JASA, 36(2).
//! - Fitzgibbons, P.J. & Wightman, F.L. (1982). Gap detection in normal
//!   and hearing-impaired listeners. JASA, 72(3).

use cricket_brain::brain::CricketBrain;

const CARRIER_FREQ: f32 = 4500.0;
const PRE_GAP_DURATION: usize = 50;  // Build up resonance
const POST_GAP_DURATION: usize = 80; // Check recovery
const N_TRIALS_PER_GAP: usize = 10;

#[allow(dead_code)]
struct GapResult {
    gap_ms: usize,
    detected: bool,
    detection_rate: f64,
    recovery_time: f64,
    pre_gap_output: f64,
    during_gap_output: f64,
    post_gap_output: f64,
}

fn run_gap_trial(brain: &mut CricketBrain, gap_ms: usize) -> (bool, usize, f32, f32, f32) {
    brain.reset();

    // Phase 1: Pre-gap tone (establish resonance)
    let mut pre_outputs = Vec::new();
    for _ in 0..PRE_GAP_DURATION {
        let out = brain.step(CARRIER_FREQ);
        pre_outputs.push(out);
    }

    // Phase 2: Silent gap
    let mut gap_outputs = Vec::new();
    for _ in 0..gap_ms {
        let out = brain.step(0.0);
        gap_outputs.push(out);
    }

    // Phase 3: Post-gap tone (measure recovery)
    let mut post_outputs = Vec::new();
    let mut first_post_spike = None;
    for i in 0..POST_GAP_DURATION {
        let out = brain.step(CARRIER_FREQ);
        post_outputs.push(out);
        if out > 0.0 && first_post_spike.is_none() {
            first_post_spike = Some(i);
        }
    }

    let pre_mean = pre_outputs.iter().copied().sum::<f32>() / pre_outputs.len().max(1) as f32;
    let gap_mean = if gap_outputs.is_empty() {
        pre_mean // no gap
    } else {
        gap_outputs.iter().copied().sum::<f32>() / gap_outputs.len() as f32
    };
    let post_mean = post_outputs.iter().copied().sum::<f32>() / post_outputs.len().max(1) as f32;

    // Gap detected if output drops significantly during gap
    let detected = gap_ms > 0 && gap_mean < pre_mean * 0.3;
    let recovery = first_post_spike.unwrap_or(POST_GAP_DURATION);

    (detected, recovery, pre_mean, gap_mean, post_mean)
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Temporal Gap Detection Benchmark                          ║");
    println!("║  Plomp (1964) / Fitzgibbons & Wightman (1982) paradigm     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut brain = CricketBrain::new(Default::default()).unwrap();

    let gap_durations = [0, 1, 2, 3, 4, 5, 6, 8, 10, 15, 20, 30, 50];
    let mut results = Vec::new();

    println!(
        "  {:>6} {:>8} {:>8} {:>10} {:>10} {:>10} {:>10}",
        "Gap", "Det?", "Rate", "Pre-Gap", "During", "Post-Gap", "Recovery"
    );
    println!(
        "  {:>6} {:>8} {:>8} {:>10} {:>10} {:>10} {:>10}",
        "──────", "────────", "────────", "──────────", "──────────", "──────────", "──────────"
    );

    for &gap in &gap_durations {
        let mut n_detected = 0;
        let mut total_recovery = 0.0;
        let mut total_pre = 0.0;
        let mut total_gap = 0.0;
        let mut total_post = 0.0;

        for _ in 0..N_TRIALS_PER_GAP {
            let (det, rec, pre, during, post) = run_gap_trial(&mut brain, gap);
            if det { n_detected += 1; }
            total_recovery += rec as f64;
            total_pre += pre as f64;
            total_gap += during as f64;
            total_post += post as f64;
        }

        let rate = n_detected as f64 / N_TRIALS_PER_GAP as f64;
        let avg_rec = total_recovery / N_TRIALS_PER_GAP as f64;
        let avg_pre = total_pre / N_TRIALS_PER_GAP as f64;
        let avg_gap = total_gap / N_TRIALS_PER_GAP as f64;
        let avg_post = total_post / N_TRIALS_PER_GAP as f64;
        let detected = rate > 0.5;

        results.push(GapResult {
            gap_ms: gap,
            detected,
            detection_rate: rate,
            recovery_time: avg_rec,
            pre_gap_output: avg_pre,
            during_gap_output: avg_gap,
            post_gap_output: avg_post,
        });

        println!(
            "  {:>4}ms {:>8} {:>7.0}% {:>10.4} {:>10.4} {:>10.4} {:>8.1}ms",
            gap,
            if detected { "YES" } else { "NO" },
            rate * 100.0,
            avg_pre, avg_gap, avg_post, avg_rec
        );
    }

    // Find minimum detectable gap
    let mdg = results.iter().find(|r| r.detected).map(|r| r.gap_ms);

    // Gap detection transfer function
    println!("\n─── Gap Detection Transfer Function ───\n");
    println!("  Gap (ms) │ Detection Rate │ Bar");
    println!("  ─────────┼────────────────┼─────────────────────");
    for r in &results {
        let bar_len = (r.detection_rate * 30.0) as usize;
        let bar: String = "#".repeat(bar_len);
        println!("  {:>5} ms │     {:>5.1}%     │ {bar}", r.gap_ms, r.detection_rate * 100.0);
    }

    // Recovery time analysis
    println!("\n─── Post-Gap Recovery Analysis ───\n");
    println!("  Gap (ms) │ Recovery (ms) │ Bar");
    println!("  ─────────┼───────────────┼─────────────────────");
    for r in &results {
        if r.gap_ms > 0 {
            let bar_len = (r.recovery_time / 2.0).min(30.0) as usize;
            let bar: String = "=".repeat(bar_len);
            println!("  {:>5} ms │   {:>7.1} ms  │ {bar}", r.gap_ms, r.recovery_time);
        }
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Gap Detection Summary                                     ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    if let Some(mdg_val) = mdg {
        println!("║  Minimum detectable gap: {mdg_val} ms                              ║");
    } else {
        println!("║  Minimum detectable gap: NOT FOUND (gaps too short)        ║");
    }
    println!("║  Human MDG at 4 kHz:     2-3 ms (Plomp, 1964)              ║");
    println!("║  Cricket biology:        ~5 ms IPI (Pollack, 1998)          ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  The system uses aggressive decay during silence            ║");
    println!("║  (amplitude *= 0.5/step), producing fast gap detection.     ║");
    println!("║  Recovery depends on coincidence window re-filling (~4ms).  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
