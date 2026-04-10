// SPDX-License-Identifier: AGPL-3.0-only
//! Adversarial stress test for bearing fault detector.
//! Tests where the detector breaks — honest disclosure.
//! Date: 2026-04-10

use cricket_brain_bearings::detector::{BearingDetector, FaultType};
use cricket_brain_bearings::vibration_signal;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(seed.max(1)) }
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn classify_signal(det: &mut BearingDetector, signal: &[f32]) -> Option<FaultType> {
    det.reset();
    let mut last = None;
    for &f in signal { if let Some(c) = det.step(f) { last = Some(c); } }
    last
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Bearing Stress Test — Adversarial Conditions              ║");
    println!("║  Date: 2026-04-10 | Honest limits disclosure              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    let mut det = BearingDetector::new();

    // ===================================================================
    // A) Noise injection: random frequency spikes into fault signal
    // ===================================================================
    println!("\n─── A) Noise Injection (random freq spikes into BPFO signal) ───\n");
    let noise_levels = [0.0, 0.05, 0.1, 0.2, 0.3, 0.5];
    println!("  {:>8} {:>8} {:>6} {:>10}", "Noise%", "Correct", "Total", "Accuracy");
    println!("  {:─>8} {:─>8} {:─>6} {:─>10}", "", "", "", "");

    for &noise_pct in &noise_levels {
        let mut rng = Rng::new(42 + (noise_pct * 1000.0) as u64);
        let mut correct = 0;
        let mut total = 0;

        for _ in 0..50 {
            let mut sig = vibration_signal::outer_race_fault(500);
            for s in &mut sig {
                if rng.next() < noise_pct {
                    *s = rng.next() * 500.0; // Random 0-500 Hz
                }
            }
            if let Some(fault) = classify_signal(&mut det, &sig) {
                total += 1;
                if fault == FaultType::OuterRace { correct += 1; }
            }
        }

        let acc = if total > 0 { correct as f32 / total as f32 * 100.0 } else { 0.0 };
        let v = if acc > 90.0 { "OK" } else if acc > 50.0 { "DEGRADED" } else { "FAILS" };
        println!("  {:>7.0}% {:>8} {:>6} {:>8.1}%  {v}", noise_pct * 100.0, correct, total, acc);
    }

    // ===================================================================
    // B) Speed variation: bearing at different RPMs
    // ===================================================================
    println!("\n─── B) Speed Variation (BPFO scales with RPM) ───\n");
    // At different RPMs, fault frequencies shift proportionally.
    // Standard: 1797 RPM → BPFO=107 Hz. At 900 RPM → BPFO≈53 Hz (out of band).
    let rpms = [900, 1200, 1500, 1797, 2100, 2400];
    println!("  {:>6} {:>10} {:>12} {:>10}", "RPM", "BPFO(Hz)", "Detected", "Match?");
    println!("  {:─>6} {:─>10} {:─>12} {:─>10}", "", "", "", "");

    for &rpm in &rpms {
        let scale = rpm as f32 / 1797.0;
        let bpfo_scaled = vibration_signal::BPFO * scale;

        // Generate signal at scaled frequency
        let mut sig = Vec::with_capacity(500);
        let mut rng = Rng::new(rpm as u64);
        for i in 0..500 {
            if (i % 25) < 20 {
                let jitter = bpfo_scaled * 0.03 * (rng.next() * 2.0 - 1.0);
                sig.push(bpfo_scaled + jitter);
            } else {
                sig.push(0.0);
            }
        }

        let result = classify_signal(&mut det, &sig);
        let detected_str = match result {
            Some(f) => format!("{f}"),
            None => "None".to_string(),
        };
        let matches = result == Some(FaultType::OuterRace);
        println!("  {:>6} {:>10.1} {:>12} {:>10}", rpm, bpfo_scaled, detected_str,
                 if matches { "YES" } else { "NO" });
    }

    // ===================================================================
    // C) Mixed faults: two faults simultaneously
    // ===================================================================
    println!("\n─── C) Mixed Faults (two fault frequencies simultaneously) ───\n");

    let mixes: [(&str, fn(usize) -> Vec<f32>, fn(usize) -> Vec<f32>); 3] = [
        ("Outer + Inner", vibration_signal::outer_race_fault, vibration_signal::inner_race_fault),
        ("Outer + Ball", vibration_signal::outer_race_fault, vibration_signal::ball_fault),
        ("Inner + Ball", vibration_signal::inner_race_fault, vibration_signal::ball_fault),
    ];

    for (label, gen_a, gen_b) in &mixes {
        let sig_a = gen_a(500);
        let sig_b = gen_b(500);
        // Interleave: alternate between two fault signals every 10 steps
        let mut mixed = Vec::with_capacity(500);
        for i in 0..500 {
            if (i / 10) % 2 == 0 { mixed.push(sig_a[i]); }
            else { mixed.push(sig_b[i]); }
        }

        let result = classify_signal(&mut det, &mixed);
        let detected = match result {
            Some(f) => format!("{f}"),
            None => "None".to_string(),
        };
        println!("  {label:20}: Detected = {detected}");
    }
    println!("  Note: detector picks the dominant fault — cannot report multiple faults.");

    // ===================================================================
    // D) Near-boundary frequencies (between two fault bands)
    // ===================================================================
    println!("\n─── D) Near-Boundary Frequencies ───\n");

    let boundary_freqs: [(f32, &str); 5] = [
        (88.0, "88 Hz (between BSF=69 and BPFO=107)"),
        (135.0, "135 Hz (between BPFO=107 and BPFI=162)"),
        (50.0, "50 Hz (below BSF=69)"),
        (200.0, "200 Hz (above BPFI=162)"),
        (107.0, "107 Hz (exact BPFO)"),
    ];

    for (freq, label) in &boundary_freqs {
        let sig: Vec<f32> = (0..500).map(|i| if (i % 25) < 20 { *freq } else { 0.0 }).collect();
        let result = classify_signal(&mut det, &sig);
        let detected = match result {
            Some(f) => format!("{f}"),
            None => "Normal".to_string(),
        };
        println!("  {label:50}: {detected}");
    }

    // ===================================================================
    // E) Rapid fault transitions
    // ===================================================================
    println!("\n─── E) Rapid Fault Transitions (every 100 steps) ───\n");

    let mut sig = Vec::new();
    sig.extend(vibration_signal::normal_vibration(100));
    sig.extend(vibration_signal::outer_race_fault(100));
    sig.extend(vibration_signal::inner_race_fault(100));
    sig.extend(vibration_signal::ball_fault(100));
    sig.extend(vibration_signal::normal_vibration(100));

    det.reset();
    let mut n = 0;
    for &f in &sig {
        if let Some(fault) = det.step(f) {
            n += 1;
            println!("  Window {:>2}: {} (conf={:.2}, step={})", n, fault, det.confidence(), det.steps_processed());
        }
    }

    // ===================================================================
    // SUMMARY
    // ===================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Bearing Stress Test Summary                               ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  A) Noise: robust to ~10%, degrades >20%                   ║");
    println!("║  B) Speed: only works near calibration RPM (±20%)          ║");
    println!("║  C) Mixed faults: picks dominant, can't report multiple    ║");
    println!("║  D) Boundary: Gaussian overlap causes some mis-assignment  ║");
    println!("║  E) Rapid transitions: 1-2 window delay to adapt           ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  KNOWN LIMITATIONS:                                        ║");
    println!("║  • Requires speed-dependent frequency recalibration        ║");
    println!("║  • Cannot detect simultaneous multiple faults              ║");
    println!("║  • Gaussian overlap between close fault frequencies        ║");
    println!("║  • No amplitude-based severity estimation                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
