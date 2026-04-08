//! # Spike Latency Benchmark
//!
//! Measures onset latency, jitter, and temporal precision of the system.
//! Standard metrics in computational neuroscience (Gerstner & Kistler, 2002).
//!
//! ## Metrics
//! - **First-spike latency**: Timesteps from signal onset to first ON1 spike
//! - **Latency jitter**: Standard deviation of first-spike times across trials
//! - **Coefficient of Variation (CV)**: jitter / mean_latency (< 0.1 = precise)
//! - **Temporal precision**: Reciprocal of jitter
//!
//! ## Reference Values
//! - Cortical neurons: 5-50 ms first-spike latency, CV 0.1-0.5
//! - Auditory nerve: 1-5 ms, CV < 0.1 (very precise)
//! - Cricket AN1: ~2 ms, CV ~0.05 (Hennig et al., 2004)
//!
//! ## Reference
//! - Gerstner, W. & Kistler, W.M. (2002). Spiking Neuron Models. Cambridge UP.
//! - Hennig, R.M. et al. (2004). Auditory interneurons in the cricket. JCP-A.

use cricket_brain::brain::CricketBrain;
use std::time::Instant;

const N_TRIALS: usize = 200;

fn measure_first_spike(brain: &mut CricketBrain, freq: f32, max_steps: usize) -> Option<usize> {
    brain.reset();
    for step in 0..max_steps {
        let out = brain.step(freq);
        if out > 0.0 {
            return Some(step);
        }
    }
    None
}

fn stats(data: &[f64]) -> (f64, f64, f64, f64, f64) {
    let n = data.len() as f64;
    let mean = data.iter().sum::<f64>() / n;
    let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    let std = variance.sqrt();
    let min = data.iter().copied().fold(f64::INFINITY, f64::min);
    let max = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (mean, std, min, max, std / mean.max(0.001))
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Spike Latency & Temporal Precision Benchmark              ║");
    println!("║  Gerstner & Kistler (2002) metrics                         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut brain = CricketBrain::new(Default::default()).unwrap();

    // === Latency at different frequencies ===
    let test_freqs: [(f32, &str); 6] = [
        (4500.0, "Target (4500 Hz)"),
        (4300.0, "Near (-4.4%)"),
        (4700.0, "Near (+4.4%)"),
        (4050.0, "Boundary (-10%)"),
        (4950.0, "Boundary (+10%)"),
        (2000.0, "Off-target"),
    ];

    println!("─── First-Spike Latency by Frequency ───\n");
    println!(
        "  {:>20} {:>6} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Condition", "N", "Mean", "SD", "Min", "Max", "CV"
    );
    println!(
        "  {:>20} {:>6} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "────────────────────", "──────", "────────", "────────", "────────", "────────", "────────"
    );

    for (freq, label) in &test_freqs {
        let mut latencies = Vec::new();
        for _ in 0..N_TRIALS {
            if let Some(lat) = measure_first_spike(&mut brain, *freq, 200) {
                latencies.push(lat as f64);
            }
        }

        if latencies.is_empty() {
            println!(
                "  {:>20} {:>6} {:>8} {:>8} {:>8} {:>8} {:>8}",
                label, N_TRIALS, "NO SPIKE", "—", "—", "—", "—"
            );
        } else {
            let (mean, sd, min, max, cv) = stats(&latencies);
            println!(
                "  {:>20} {:>6} {:>6.1}ms {:>6.2}ms {:>6.0}ms {:>6.0}ms {:>8.4}",
                label,
                latencies.len(),
                mean, sd, min, max, cv
            );
        }
    }

    // === Temporal precision: repeated identical stimuli ===
    println!("\n─── Temporal Precision (Repeated Identical Stimuli) ───\n");

    let mut latencies_4500 = Vec::new();
    let mut wall_times = Vec::new();

    for _ in 0..1000 {
        let t0 = Instant::now();
        if let Some(lat) = measure_first_spike(&mut brain, 4500.0, 200) {
            latencies_4500.push(lat as f64);
            wall_times.push(t0.elapsed().as_nanos() as f64);
        }
    }

    if !latencies_4500.is_empty() {
        let (mean, sd, min, max, cv) = stats(&latencies_4500);
        let (wmean, wsd, _, _, _) = stats(&wall_times);

        println!("  Trials:             1000");
        println!("  Spike prob:         {:.1}%", latencies_4500.len() as f64 / 10.0);
        println!("  Mean latency:       {mean:.2} ms (simulated)");
        println!("  Jitter (SD):        {sd:.3} ms");
        println!("  CV:                 {cv:.4}");
        println!("  Min/Max:            {min:.0} / {max:.0} ms");
        println!("  Wall-clock/trial:   {:.1} us", wmean / 1000.0);
        println!("  Wall-clock jitter:  {:.1} us", wsd / 1000.0);

        let precision_rating = if cv < 0.05 {
            "EXCELLENT (< 0.05, auditory-nerve level)"
        } else if cv < 0.1 {
            "GOOD (< 0.1, brainstem level)"
        } else if cv < 0.3 {
            "MODERATE (< 0.3, cortical level)"
        } else {
            "POOR (> 0.3)"
        };
        println!("  Precision rating:   {precision_rating}");
    }

    // === Inter-spike interval (ISI) analysis during sustained tone ===
    println!("\n─── Inter-Spike Interval Analysis (Sustained 4500 Hz) ───\n");

    brain.reset();
    let mut spike_times = Vec::new();
    for step in 0..500 {
        let out = brain.step(4500.0);
        if out > 0.0 {
            spike_times.push(step);
        }
    }

    if spike_times.len() > 1 {
        let isis: Vec<f64> = spike_times.windows(2).map(|w| (w[1] - w[0]) as f64).collect();
        let (isi_mean, isi_sd, isi_min, isi_max, isi_cv) = stats(&isis);

        println!("  Duration:           500 ms");
        println!("  Total spikes:       {}", spike_times.len());
        println!("  Mean ISI:           {isi_mean:.2} ms");
        println!("  ISI SD:             {isi_sd:.3} ms");
        println!("  ISI CV:             {isi_cv:.4}");
        println!("  ISI range:          {isi_min:.0} – {isi_max:.0} ms");
        println!("  Firing rate:        {:.1} Hz", 1000.0 / isi_mean);

        // ISI regularity classification
        let regularity = if isi_cv < 0.01 {
            "CLOCK-LIKE (CV < 0.01)"
        } else if isi_cv < 0.1 {
            "REGULAR (CV < 0.1)"
        } else if isi_cv < 0.5 {
            "IRREGULAR"
        } else {
            "POISSON-LIKE (CV ~ 1.0)"
        };
        println!("  ISI pattern:        {regularity}");
    }

    // === Summary ===
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Latency Benchmark Summary                                 ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Cricket AN1 biology: ~2 ms latency, CV ~0.05              ║");
    println!("║  This model: deterministic (CV = 0 for identical input)    ║");
    println!("║  Real jitter would require stochastic noise injection      ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Ref: Hennig et al. (2004) J Comp Physiol A               ║");
    println!("║  Ref: Gerstner & Kistler (2002) Spiking Neuron Models     ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
