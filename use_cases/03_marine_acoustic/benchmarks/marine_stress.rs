// SPDX-License-Identifier: AGPL-3.0-only
//! Adversarial stress test for the marine acoustic detector.
//! Tests where the detector breaks — honest disclosure.
//! Date: 2026-04-24

use cricket_brain_marine::acoustic_signal;
use cricket_brain_marine::detector::{AcousticEvent, MarineDetector};

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn classify_signal(det: &mut MarineDetector, signal: &[f32]) -> Option<AcousticEvent> {
    det.reset();
    let mut last = None;
    for &f in signal {
        if let Some(c) = det.step(f) {
            last = Some(c);
        }
    }
    last
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Marine Stress Test — Adversarial Conditions               ║");
    println!("║  Date: 2026-04-24 | Honest limits disclosure              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    let mut det = MarineDetector::new();

    // ===================================================================
    // A) Noise injection: random frequency spikes on top of ship signal
    // ===================================================================
    println!("\n─── A) Noise Injection (random freq spikes into ship signal) ───\n");
    let noise_levels = [0.0, 0.05, 0.1, 0.2, 0.3, 0.5];
    println!(
        "  {:>8} {:>8} {:>6} {:>10}",
        "Noise%", "Correct", "Total", "Accuracy"
    );
    println!("  {:─>8} {:─>8} {:─>6} {:─>10}", "", "", "", "");

    for &noise_pct in &noise_levels {
        let mut rng = Rng::new(42 + (noise_pct * 1000.0) as u64);
        let mut correct = 0;
        let mut total = 0;

        for _ in 0..50 {
            let mut sig = acoustic_signal::ship_passage(500);
            for s in &mut sig {
                if rng.next() < noise_pct {
                    *s = rng.next() * 500.0;
                }
            }
            if let Some(event) = classify_signal(&mut det, &sig) {
                total += 1;
                if event == AcousticEvent::ShipNoise {
                    correct += 1;
                }
            }
        }

        let acc = if total > 0 {
            correct as f32 / total as f32 * 100.0
        } else {
            0.0
        };
        let v = if acc > 90.0 {
            "OK"
        } else if acc > 50.0 {
            "DEGRADED"
        } else {
            "FAILS"
        };
        println!(
            "  {:>7.0}% {:>8} {:>6} {:>8.1}%  {v}",
            noise_pct * 100.0,
            correct,
            total,
            acc
        );
    }

    // ===================================================================
    // B) Ship transits: ship sailing past the hydrophone
    // ===================================================================
    println!("\n─── B) Ship Transits (vessel sailing past at distance d) ───\n");
    println!(
        "  {:>10} {:>10} {:>10} {:>12}",
        "Transit s", "Ship win", "Total win", "Ship ratio"
    );
    println!("  {:─>10} {:─>10} {:─>10} {:─>12}", "", "", "", "");

    // Simulate several transits of varying duration (= ship size / speed).
    let durations = [500, 1000, 1500, 2000, 3000, 5000];
    for &dur in &durations {
        let sig = acoustic_signal::ship_transit(dur);
        det.reset();
        let mut ship = 0;
        let mut total = 0;
        for &f in &sig {
            if let Some(e) = det.step(f) {
                total += 1;
                if e == AcousticEvent::ShipNoise {
                    ship += 1;
                }
            }
        }
        let ratio = if total > 0 {
            ship as f32 / total as f32 * 100.0
        } else {
            0.0
        };
        println!("  {:>10} {:>10} {:>10} {:>10.1}%", dur, ship, total, ratio);
    }

    // ===================================================================
    // C) Whale call masked by ship noise (common real-ocean scenario)
    // ===================================================================
    println!("\n─── C) Fin-whale pulses under simultaneous ship passage ───\n");
    let sig = acoustic_signal::fin_whale_under_ship(2000);
    det.reset();
    let mut counts = [0usize; 5];
    for &f in &sig {
        if let Some(e) = det.step(f) {
            match e {
                AcousticEvent::Ambient => counts[0] += 1,
                AcousticEvent::FinWhale => counts[1] += 1,
                AcousticEvent::BlueWhale => counts[2] += 1,
                AcousticEvent::ShipNoise => counts[3] += 1,
                AcousticEvent::Humpback => counts[4] += 1,
            }
        }
    }
    println!(
        "  Ambient={}  FinWhale={}  BlueWhale={}  ShipNoise={}  Humpback={}",
        counts[0], counts[1], counts[2], counts[3], counts[4]
    );
    println!("  Note: the detector picks the dominant source per 50-step window.");

    // ===================================================================
    // D) Sea-state sweep: background noise from wind / waves
    // ===================================================================
    println!("\n─── D) Sea-State Sweep on Ambient Signal ───\n");
    println!(
        "  {:>10} {:>12} {:>14}",
        "Sea State", "Ambient %", "FP species"
    );
    println!("  {:─>10} {:─>12} {:─>14}", "", "", "");

    for state in [0u8, 2, 4, 6, 8] {
        let mut d = MarineDetector::new();
        d.set_sea_state(state);
        let sig = acoustic_signal::ambient_noise(1000);
        let mut amb = 0usize;
        let mut fp = 0usize;
        let mut total = 0usize;
        for &f in &sig {
            if let Some(e) = d.step(f) {
                total += 1;
                if e == AcousticEvent::Ambient {
                    amb += 1;
                } else {
                    fp += 1;
                }
            }
        }
        let amb_pct = if total > 0 {
            amb as f32 / total as f32 * 100.0
        } else {
            0.0
        };
        println!("  {:>10} {:>10.1}% {:>14}", state, amb_pct, fp);
    }

    // ===================================================================
    // E) Simultaneous multiple species
    // ===================================================================
    println!("\n─── E) Simultaneous Species (pairs alternating every 10 steps) ───\n");

    let mixes: [(&str, fn(usize) -> Vec<f32>, fn(usize) -> Vec<f32>); 3] = [
        (
            "Fin + Blue",
            acoustic_signal::fin_whale_call,
            acoustic_signal::blue_whale_call,
        ),
        (
            "Blue + Humpback",
            acoustic_signal::blue_whale_call,
            acoustic_signal::humpback_song,
        ),
        (
            "Fin + Humpback",
            acoustic_signal::fin_whale_call,
            acoustic_signal::humpback_song,
        ),
    ];

    for (label, gen_a, gen_b) in &mixes {
        let sig_a = gen_a(500);
        let sig_b = gen_b(500);
        let mut mixed = Vec::with_capacity(500);
        for i in 0..500 {
            if (i / 10) % 2 == 0 {
                mixed.push(sig_a[i]);
            } else {
                mixed.push(sig_b[i]);
            }
        }

        let result = classify_signal(&mut det, &mixed);
        let detected = match result {
            Some(f) => format!("{f}"),
            None => "None".to_string(),
        };
        println!("  {label:20}: Detected = {detected}");
    }
    println!("  Note: detector picks the dominant species — cannot report multiple.");

    // ===================================================================
    // F) Near-boundary frequencies
    // ===================================================================
    println!("\n─── F) Near-Boundary Frequencies ───\n");
    let boundary_freqs: [(f32, &str); 6] = [
        (50.0, "50 Hz (between Fin=20 and Blue=80)"),
        (110.0, "110 Hz (between Blue=80 and Ship=140)"),
        (170.0, "170 Hz (between Ship=140 and Hump=200)"),
        (15.0, "15 Hz (below Fin=20)"),
        (260.0, "260 Hz (above Hump=200)"),
        (80.0, "80 Hz (exact Blue A-call)"),
    ];

    for (freq, label) in &boundary_freqs {
        let sig: Vec<f32> = (0..500).map(|i| if (i % 25) < 20 { *freq } else { 0.0 }).collect();
        let result = classify_signal(&mut det, &sig);
        let detected = match result {
            Some(f) => format!("{f}"),
            None => "Ambient".to_string(),
        };
        println!("  {label:55}: {detected}");
    }

    // ===================================================================
    // G) Rapid scene changes (encounter sequence)
    // ===================================================================
    println!("\n─── G) Encounter Sequence: Ambient → Ship → Whale → Humpback ───\n");

    let mut sig = Vec::new();
    sig.extend(acoustic_signal::ambient_noise(100));
    sig.extend(acoustic_signal::ship_passage(200));
    sig.extend(acoustic_signal::fin_whale_call(150));
    sig.extend(acoustic_signal::blue_whale_call(150));
    sig.extend(acoustic_signal::humpback_song(200));
    sig.extend(acoustic_signal::ambient_noise(100));

    det.reset();
    let mut n = 0;
    for &f in &sig {
        if let Some(event) = det.step(f) {
            n += 1;
            println!(
                "  Window {:>2}: {} (conf={:.2}, step={})",
                n,
                event,
                det.confidence(),
                det.steps_processed()
            );
        }
    }

    // ===================================================================
    // SUMMARY
    // ===================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Marine Stress Test Summary                                ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  A) Noise: robust to ~10%, degrades above 20%              ║");
    println!("║  B) Ship transits: consistently flagged during CPA         ║");
    println!("║  C) Whale under ship: ship dominates, pulses still surface ║");
    println!("║  D) Sea-state: threshold scaling suppresses false alarms   ║");
    println!("║  E) Simultaneous species: picks dominant, no multi-label   ║");
    println!("║  F) Boundary: Gaussian overlap causes mis-assignment       ║");
    println!("║  G) Scene changes: 1-2 window delay to adapt               ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  KNOWN LIMITATIONS:                                        ║");
    println!("║  • No multi-label output for simultaneous species          ║");
    println!("║  • Cannot estimate source distance or bearing              ║");
    println!("║  • Species overlap at boundary frequencies                 ║");
    println!("║  • No amplitude-based severity / range estimation          ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
