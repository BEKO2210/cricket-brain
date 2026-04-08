//! # Pattern Separation & Completion Benchmark
//!
//! Standard memory circuit evaluation from hippocampal research,
//! adapted for the temporal pattern domain.
//! (Yassa & Stark, 2011; Leutgeb et al., 2007)
//!
//! ## Pattern Separation
//! How well does the system distinguish similar but different input patterns?
//! Measured as output dissimilarity given input similarity.
//! - Present pairs of patterns with controlled overlap
//! - Measure Hamming distance / correlation of output spike trains
//! - Perfect separation: low input distance → high output distance
//!
//! ## Pattern Completion
//! Can the system recognize a partial/degraded version of a known pattern?
//! - Train on full pattern, test with truncated/noisy version
//! - Measure: does output match full-pattern response?
//!
//! ## Metrics
//! - **Separation Index**: output_distance / input_distance
//!   - > 1.0: orthogonalization (amplifies differences)
//!   - = 1.0: linear mapping
//!   - < 1.0: compression (generalizing)
//!
//! ## References
//! - Yassa, M.A. & Stark, C.E.L. (2011). Pattern separation in the
//!   hippocampus. Trends in Neurosciences, 34(10).
//! - Leutgeb, J.K. et al. (2007). Pattern separation in the dentate
//!   gyrus and CA3 of the hippocampus. Science, 315(5814).

use cricket_brain::brain::CricketBrain;

const MORSE_FREQ: f32 = 4500.0;
const DOT_MS: usize = 50;
const DASH_MS: usize = 150;
const GAP_MS: usize = 50;
#[allow(dead_code)]
const CHAR_GAP_MS: usize = 150;

/// Generate a temporal pattern as (freq, duration) pairs from a Morse-like encoding.
fn make_pattern(elements: &[bool]) -> Vec<(f32, usize)> {
    let mut signal = Vec::new();
    for (i, &is_dash) in elements.iter().enumerate() {
        signal.push((MORSE_FREQ, if is_dash { DASH_MS } else { DOT_MS }));
        if i + 1 < elements.len() {
            signal.push((0.0, GAP_MS));
        }
    }
    signal
}

/// Run a pattern through the brain, return spike times.
fn collect_spikes(brain: &mut CricketBrain, pattern: &[(f32, usize)]) -> Vec<usize> {
    brain.reset();
    let mut spikes = Vec::new();
    let mut t = 0;
    for &(freq, dur) in pattern {
        for _ in 0..dur {
            let out = brain.step(freq);
            if out > 0.0 {
                spikes.push(t);
            }
            t += 1;
        }
    }
    spikes
}

/// Hamming distance between two binary spike trains (normalized).
fn spike_train_distance(a: &[usize], b: &[usize], total_steps: usize) -> f64 {
    let mut train_a = vec![false; total_steps];
    let mut train_b = vec![false; total_steps];
    for &t in a { if t < total_steps { train_a[t] = true; } }
    for &t in b { if t < total_steps { train_b[t] = true; } }

    let diffs = train_a.iter().zip(&train_b).filter(|(&x, &y)| x != y).count();
    diffs as f64 / total_steps as f64
}

/// Input distance: fraction of elements that differ between two patterns.
fn pattern_distance(a: &[bool], b: &[bool]) -> f64 {
    let max_len = a.len().max(b.len());
    if max_len == 0 { return 0.0; }
    let diffs = a.iter().zip(b.iter()).filter(|(&x, &y)| x != y).count();
    let len_diff = (a.len() as isize - b.len() as isize).unsigned_abs();
    (diffs + len_diff) as f64 / max_len as f64
}

fn total_duration(pattern: &[(f32, usize)]) -> usize {
    pattern.iter().map(|&(_, d)| d).sum()
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Pattern Separation & Completion Benchmark                 ║");
    println!("║  Yassa & Stark (2011) / Leutgeb et al. (2007) paradigm     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut brain = CricketBrain::new(Default::default()).unwrap();

    // === Pattern Separation ===
    println!("─── Pattern Separation ───\n");
    println!("  Measuring output dissimilarity for systematically varied inputs.\n");

    // Reference pattern: ... (S in Morse) = [dot, dot, dot]
    let reference = vec![false, false, false]; // S: dot dot dot

    // Variants with increasing distance from reference
    let variants: Vec<(&str, Vec<bool>)> = vec![
        ("... (identical)", vec![false, false, false]),       // distance 0
        (".-. (1 change)",  vec![false, true, false]),        // distance 1/3
        ("--. (2 changes)", vec![true, true, false]),          // distance 2/3
        ("--- (all diff)",  vec![true, true, true]),           // distance 3/3
        (".. (shorter)",    vec![false, false]),                // structural diff
        (".... (longer)",   vec![false, false, false, false]), // structural diff
    ];

    let ref_pattern = make_pattern(&reference);
    let ref_spikes = collect_spikes(&mut brain, &ref_pattern);
    let ref_dur = total_duration(&ref_pattern);

    println!(
        "  {:>16} {:>10} {:>12} {:>12} {:>10}",
        "Pattern", "InputDist", "OutputDist", "SepIndex", "Spikes"
    );
    println!(
        "  {:>16} {:>10} {:>12} {:>12} {:>10}",
        "────────────────", "──────────", "────────────", "────────────", "──────────"
    );

    let mut separation_indices = Vec::new();

    for (label, variant) in &variants {
        let var_pattern = make_pattern(variant);
        let var_spikes = collect_spikes(&mut brain, &var_pattern);
        let max_dur = ref_dur.max(total_duration(&var_pattern));

        let input_dist = pattern_distance(&reference, variant);
        let output_dist = spike_train_distance(&ref_spikes, &var_spikes, max_dur);
        let sep_index = if input_dist > 0.001 {
            output_dist / input_dist
        } else {
            if output_dist < 0.001 { 1.0 } else { f64::INFINITY }
        };

        if input_dist > 0.001 {
            separation_indices.push(sep_index);
        }

        println!(
            "  {:>16} {:>10.3} {:>12.4} {:>12.3} {:>10}",
            label, input_dist, output_dist, sep_index, var_spikes.len()
        );
    }

    let mean_sep = if separation_indices.is_empty() {
        0.0
    } else {
        separation_indices.iter().sum::<f64>() / separation_indices.len() as f64
    };

    println!("\n  Mean separation index: {mean_sep:.3}");
    println!("  Interpretation:");
    if mean_sep > 1.5 {
        println!("    ORTHOGONALIZING — system amplifies input differences");
    } else if mean_sep > 0.8 {
        println!("    LINEAR — output distance tracks input distance");
    } else {
        println!("    GENERALIZING — system compresses differences");
    }

    // === Pattern Completion ===
    println!("\n─── Pattern Completion ───\n");
    println!("  Can the system produce a similar output from truncated input?\n");

    // Full pattern: SOS = ... --- ...
    let full_sos = vec![
        false, false, false, // S
        true, true, true,    // O
        false, false, false, // S
    ];
    let full_pattern = make_pattern(&full_sos);
    let full_spikes = collect_spikes(&mut brain, &full_pattern);
    let full_dur = total_duration(&full_pattern);

    // Truncated versions
    let truncations: Vec<(&str, Vec<bool>)> = vec![
        ("S (33%)", vec![false, false, false]),
        ("SO (67%)", vec![false, false, false, true, true, true]),
        ("SOS (100%)", full_sos.clone()),
    ];

    println!(
        "  {:>12} {:>8} {:>12} {:>12}",
        "Prefix", "Spikes", "Correlation", "Completion%"
    );
    println!(
        "  {:>12} {:>8} {:>12} {:>12}",
        "────────────", "────────", "────────────", "────────────"
    );

    for (label, prefix) in &truncations {
        let prefix_pattern = make_pattern(prefix);
        let prefix_spikes = collect_spikes(&mut brain, &prefix_pattern);

        // Correlation: fraction of full-pattern spike positions that also fire in prefix
        let prefix_dur = total_duration(&prefix_pattern);
        let mut full_train = vec![false; full_dur];
        let mut prefix_train = vec![false; full_dur];
        for &t in &full_spikes { if t < full_dur { full_train[t] = true; } }
        for &t in &prefix_spikes { if t < full_dur { prefix_train[t] = true; } }

        let matching = full_train.iter()
            .zip(&prefix_train)
            .take(prefix_dur)
            .filter(|(&a, &b)| a == b)
            .count();
        let correlation = matching as f64 / prefix_dur.max(1) as f64;
        let completion = prefix.len() as f64 / full_sos.len() as f64 * 100.0;

        println!(
            "  {:>12} {:>8} {:>12.4} {:>11.0}%",
            label, prefix_spikes.len(), correlation, completion
        );
    }

    // === Multi-token separation (v0.2 domain) ===
    println!("\n─── Multi-Frequency Token Separation ───\n");
    println!("  Testing output orthogonality across frequency-encoded tokens.\n");

    let freqs = [2000.0_f32, 3000.0, 4000.0, 4500.0, 5000.0, 6000.0, 7000.0, 8000.0];
    let mut spike_trains: Vec<Vec<usize>> = Vec::new();
    let steps = 100;

    for &freq in &freqs {
        brain.reset();
        let mut spikes = Vec::new();
        for t in 0..steps {
            let out = brain.step(freq);
            if out > 0.0 { spikes.push(t); }
        }
        spike_trains.push(spikes);
    }

    // Print pairwise distance matrix
    print!("  {:>6}", "Hz");
    for &f in &freqs { print!(" {:>6.0}", f); }
    println!();
    print!("  {:>6}", "──────");
    for _ in &freqs { print!(" {:>6}", "──────"); }
    println!();

    for (i, &fi) in freqs.iter().enumerate() {
        print!("  {:>6.0}", fi);
        for (j, _) in freqs.iter().enumerate() {
            let dist = spike_train_distance(&spike_trains[i], &spike_trains[j], steps);
            print!(" {:>6.3}", dist);
        }
        println!();
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Pattern Benchmark Summary                                 ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Mean separation index: {mean_sep:>6.3}                             ║");
    println!("║  System behavior: temporal pattern + frequency coding      ║");
    println!("║  Ref: Yassa & Stark (2011), Trends in Neurosciences        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
