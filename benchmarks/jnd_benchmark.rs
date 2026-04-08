//! # Just Noticeable Difference (JND) Benchmark
//!
//! Standard psychoacoustics measurement from Weber's Law and Fechner's work.
//! Measures the minimum frequency difference the system can reliably detect.
//!
//! ## Method: Adaptive Staircase (Levitt, 1971)
//! Two-interval forced-choice (2IFC): on each trial, present two intervals —
//! one at reference frequency, one at reference + delta. The system must
//! identify which interval differs. Delta adjusts up/down based on performance.
//!
//! ## Metrics
//! - **JND (Hz)**: Smallest discriminable frequency difference
//! - **Weber Fraction**: JND / reference_freq (dimensionless)
//! - **Human comparison**: Weber fraction for frequency ≈ 0.002–0.003 at 4 kHz
//!   (Moore, 2012, An Introduction to the Psychology of Hearing)
//!
//! ## Reference
//! - Levitt, H. (1971). Transformed up-down methods in psychoacoustics. JASA.
//! - Weber, E.H. (1834). De pulsu, resorptione, auditu et tactu.

use cricket_brain::brain::CricketBrain;

const REFERENCE_FREQ: f32 = 4500.0;
const TRIAL_DURATION_MS: usize = 80;
const N_REVERSALS_TARGET: usize = 12;
const INITIAL_DELTA_HZ: f32 = 500.0;
const MIN_DELTA_HZ: f32 = 1.0;
const STEP_FACTOR_DOWN: f32 = 0.707; // 2-down: reduce by √2
const STEP_FACTOR_UP: f32 = 1.414;   // 1-up: increase by √2

/// Run a single frequency through the brain, return peak ON1 amplitude.
fn measure_response(brain: &mut CricketBrain, freq: f32) -> f32 {
    brain.reset();
    let mut peak = 0.0_f32;
    for step in 0..TRIAL_DURATION_MS {
        let out = brain.step(freq);
        if step >= 10 { // skip ramp
            peak = peak.max(out);
        }
    }
    peak
}

/// 2IFC trial: present reference and test frequency, return true if system
/// correctly identifies which one is different (higher response to reference).
fn two_ifc_trial(brain: &mut CricketBrain, reference: f32, test: f32) -> bool {
    let resp_ref = measure_response(brain, reference);
    let resp_test = measure_response(brain, test);
    // System "chose correctly" if reference response > test response
    // (since the system is tuned to reference)
    resp_ref > resp_test
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Just Noticeable Difference (JND) Benchmark                ║");
    println!("║  Levitt (1971) adaptive staircase, 2IFC paradigm           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut brain = CricketBrain::new(Default::default()).unwrap();

    // Test both directions: above and below reference
    for direction_label in ["ABOVE reference (f + Δf)", "BELOW reference (f - Δf)"] {
        let above = direction_label.starts_with("ABOVE");
        println!("─── Direction: {direction_label} ───\n");

        let mut delta = INITIAL_DELTA_HZ;
        let mut reversals = 0;
        let mut last_correct = true;
        let mut consecutive_correct = 0;
        let mut reversal_deltas = Vec::new();
        let mut trial = 0;

        while reversals < N_REVERSALS_TARGET && trial < 200 {
            let test_freq = if above {
                REFERENCE_FREQ + delta
            } else {
                REFERENCE_FREQ - delta
            };

            let correct = two_ifc_trial(&mut brain, REFERENCE_FREQ, test_freq);
            trial += 1;

            if correct {
                consecutive_correct += 1;
                // 2-down rule: reduce delta after 2 consecutive correct
                if consecutive_correct >= 2 {
                    if last_correct {
                        // Check for reversal
                        reversals += 1;
                        reversal_deltas.push(delta);
                    }
                    delta = (delta * STEP_FACTOR_DOWN).max(MIN_DELTA_HZ);
                    consecutive_correct = 0;
                    last_correct = true;
                }
            } else {
                // 1-up rule: increase delta after 1 incorrect
                if !last_correct || consecutive_correct > 0 {
                    reversals += 1;
                    reversal_deltas.push(delta);
                }
                delta = (delta * STEP_FACTOR_UP).min(INITIAL_DELTA_HZ);
                consecutive_correct = 0;
                last_correct = false;
            }
        }

        // JND = average of last 8 reversals (standard Levitt procedure)
        let n_avg = 8.min(reversal_deltas.len());
        let jnd: f32 = if n_avg > 0 {
            reversal_deltas[reversal_deltas.len() - n_avg..].iter().sum::<f32>() / n_avg as f32
        } else {
            delta
        };
        let weber = jnd / REFERENCE_FREQ;

        println!("  Trials:           {trial}");
        println!("  Reversals:        {reversals}");
        println!("  JND:              {jnd:.1} Hz");
        println!("  Weber fraction:   {weber:.5} (Δf/f)");
        println!("  Weber %:          {:.2}%", weber * 100.0);
        println!();
    }

    // === Systematic frequency sweep: measure discrimination at fixed deltas ===
    println!("─── Systematic Discrimination Sweep ───\n");
    println!("  {:>10} {:>12} {:>12} {:>10}", "Delta Hz", "Ref Response", "Test Response", "Discrim?");
    println!("  {:>10} {:>12} {:>12} {:>10}", "────────", "────────────", "────────────", "────────");

    let deltas = [1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 200.0, 450.0, 900.0, 1500.0];
    let mut jnd_threshold_delta = 0.0_f32;

    for &d in &deltas {
        let test = REFERENCE_FREQ + d;
        let resp_ref = measure_response(&mut brain, REFERENCE_FREQ);
        let resp_test = measure_response(&mut brain, test);
        let discriminated = (resp_ref - resp_test).abs() > 0.01;
        if discriminated && jnd_threshold_delta == 0.0 {
            jnd_threshold_delta = d;
        }
        println!(
            "  {:>10.0} {:>12.4} {:>12.4} {:>10}",
            d, resp_ref, resp_test,
            if discriminated { "YES" } else { "NO" }
        );
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  JND Summary                                               ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    let weber_pct = if jnd_threshold_delta > 0.0 {
        jnd_threshold_delta / REFERENCE_FREQ * 100.0
    } else {
        0.0
    };
    println!("║  System JND:    ~{:.0} Hz (Weber: {:.2}%)                       ║", jnd_threshold_delta, weber_pct);
    println!("║  Human JND:     ~9 Hz at 4kHz (Weber: 0.2-0.3%)             ║");
    println!("║  Design target: ±10% bandwidth = 450 Hz (Weber: 10%)        ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  The system's wide bandwidth is intentional: it's a         ║");
    println!("║  categorical detector (on/off), not a fine discriminator.   ║");
    println!("║  This matches cricket biology: species identification,      ║");
    println!("║  not pitch perception. (Pollack, 2000, JCPA)                ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
