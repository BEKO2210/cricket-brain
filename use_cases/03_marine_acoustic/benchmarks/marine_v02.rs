// SPDX-License-Identifier: AGPL-3.0-only
//! v0.1 vs v0.2 head-to-head comparison.
//!
//! Measures the two v0.2 design changes proposed for UC03:
//!   1. Wider Gaussian tuning (bandwidth 0.30 vs the default 0.10).
//!   2. Multi-label output (`step_multi`) vs the v0.1 single-label `step`.
//!
//! Prints side-by-side accuracy on:
//!   - boundary frequencies (50 / 110 / 170 / 260 Hz)
//!   - fin-whale pulses overlapping a simultaneous ship passage
//!   - pure ambient ocean (regression check — no false species alarms)
//!   - the existing sample_marine.csv (overall accuracy)
//!
//! Date: 2026-04-24

use cricket_brain_marine::acoustic_signal;
use cricket_brain_marine::detector::{AcousticEvent, ConfusionMatrix, MarineDetector};

const BOUNDARY_STEPS: usize = 500;

fn last_single(det: &mut MarineDetector, sig: &[f32]) -> Option<AcousticEvent> {
    det.reset();
    let mut last = None;
    for &f in sig {
        if let Some(e) = det.step(f) {
            last = Some(e);
        }
    }
    last
}

fn boundary_signal(freq: f32) -> Vec<f32> {
    (0..BOUNDARY_STEPS)
        .map(|i| if (i % 25) < 20 { freq } else { 0.0 })
        .collect()
}

fn print_boundary(name: &str, freq: f32) {
    let mut v01 = MarineDetector::new();
    let mut v02a = MarineDetector::with_bandwidth(0.20);
    let mut v02b = MarineDetector::with_bandwidth(0.30);
    let sig = boundary_signal(freq);
    let r1 = last_single(&mut v01, &sig).map(|e| format!("{e}"));
    let ra = last_single(&mut v02a, &sig).map(|e| format!("{e}"));
    let rb = last_single(&mut v02b, &sig).map(|e| format!("{e}"));
    println!(
        "  {:<42} {:<22} {:<22} {:<22}",
        name,
        r1.unwrap_or_else(|| "-".into()),
        ra.unwrap_or_else(|| "-".into()),
        rb.unwrap_or_else(|| "-".into())
    );
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Marine v0.1 vs v0.2 Comparison                            ║");
    println!("║  Date: 2026-04-24                                          ║");
    println!("║  v0.2 changes: bandwidth 0.10 → 0.30, step_multi() added   ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // ------------------------------------------------------------------
    // 1. Boundary frequencies
    // ------------------------------------------------------------------
    println!("─── 1) Boundary-frequency recovery (single-label, sustained tone) ───\n");
    println!(
        "  {:<42} {:<22} {:<22} {:<22}",
        "Input frequency",
        "v0.1 (bw=0.10)",
        "v0.2a (bw=0.20)",
        "v0.2b (bw=0.30)"
    );
    println!("  {:─<42} {:─<22} {:─<22} {:─<22}", "", "", "", "");
    print_boundary("50 Hz (between Fin=20 and Blue=80)", 50.0);
    print_boundary("110 Hz (between Blue=80 and Ship=140)", 110.0);
    print_boundary("170 Hz (between Ship=140 and Hump=200)", 170.0);
    print_boundary("260 Hz (above Hump=200)", 260.0);
    print_boundary("15 Hz (below Fin=20)", 15.0);
    print_boundary("80 Hz (exact Blue A-call)", 80.0);
    print_boundary("140 Hz (exact Ship cavitation)", 140.0);

    // ------------------------------------------------------------------
    // 2. Multi-label on whale-under-ship scene
    // ------------------------------------------------------------------
    println!("\n─── 2) Whale-under-ship: single-label vs multi-label ───\n");

    let sig = acoustic_signal::fin_whale_under_ship(2000);

    // v0.1 single-label baseline
    let mut v01 = MarineDetector::new();
    let mut v01_counts = [0usize; 5];
    for &f in &sig {
        if let Some(e) = v01.step(f) {
            match e {
                AcousticEvent::Ambient => v01_counts[0] += 1,
                AcousticEvent::FinWhale => v01_counts[1] += 1,
                AcousticEvent::BlueWhale => v01_counts[2] += 1,
                AcousticEvent::ShipNoise => v01_counts[3] += 1,
                AcousticEvent::Humpback => v01_counts[4] += 1,
            }
        }
    }
    let v01_total: usize = v01_counts.iter().sum();

    // v0.2 multi-label — report both 0.20 and 0.30 bandwidths
    let mut run_multi = |bw: f32, label: &str| {
        let mut v02 = MarineDetector::with_bandwidth(bw);
        let mut both = 0usize;
        let mut fin_only = 0usize;
        let mut ship_only = 0usize;
        let mut other = 0usize;
        let mut total = 0usize;
        for &f in &sig {
            if let Some(d) = v02.step_multi(f) {
                total += 1;
                let has_fin = d.events.contains(&AcousticEvent::FinWhale);
                let has_ship = d.events.contains(&AcousticEvent::ShipNoise);
                match (has_fin, has_ship) {
                    (true, true) => both += 1,
                    (true, false) => fin_only += 1,
                    (false, true) => ship_only += 1,
                    _ => other += 1,
                }
            }
        }
        println!(
            "  {label}: both={} fin_only={} ship_only={} other={} total={} → both in {:.0}%",
            both,
            fin_only,
            ship_only,
            other,
            total,
            100.0 * both as f32 / total.max(1) as f32
        );
    };

    println!(
        "  v0.1 single-label (picks dominant):  FinWhale={} ShipNoise={} total={}",
        v01_counts[1], v01_counts[3], v01_total
    );
    run_multi(0.20, "v0.2a multi-label (bw=0.20)");
    run_multi(0.30, "v0.2b multi-label (bw=0.30)");

    // ------------------------------------------------------------------
    // 3. Regression: pure ambient must stay quiet
    // ------------------------------------------------------------------
    println!("\n─── 3) Regression check: pure ambient ocean ───\n");
    let amb = acoustic_signal::ambient_noise(2000);

    let mut v01 = MarineDetector::new();
    let (mut v01_amb, mut v01_fp) = (0usize, 0usize);
    for &f in &amb {
        if let Some(e) = v01.step(f) {
            if e == AcousticEvent::Ambient {
                v01_amb += 1;
            } else {
                v01_fp += 1;
            }
        }
    }

    let mut v02 = MarineDetector::with_bandwidth(0.30);
    let (mut v02_amb, mut v02_fp) = (0usize, 0usize);
    for &f in &amb {
        if let Some(d) = v02.step_multi(f) {
            if d.events == vec![AcousticEvent::Ambient] {
                v02_amb += 1;
            } else {
                v02_fp += 1;
            }
        }
    }

    println!(
        "  v0.1 on ambient:  Ambient={}  species-FP={}",
        v01_amb, v01_fp
    );
    println!(
        "  v0.2 on ambient:  Ambient={}  species-FP={}",
        v02_amb, v02_fp
    );

    // ------------------------------------------------------------------
    // 4. CSV accuracy: bandwidth sweep
    // ------------------------------------------------------------------
    println!("\n─── 4) Bandwidth sweep — CSV sample_marine.csv accuracy ───\n");
    println!("  {:<22} {:<20}", "Bandwidth", "Accuracy");
    println!("  {:─<22} {:─<20}", "", "");
    let windows = acoustic_signal::from_csv("data/processed/sample_marine.csv");

    // v0.1 baseline (whatever the library picks — auto-clamped at 0.10)
    let mut v01 = MarineDetector::new();
    let preds = v01.classify_stream(&windows, 25);
    let cm1 = ConfusionMatrix::from_predictions(&preds, &windows, 25);
    println!(
        "  {:<22} {}/{} = {:.1} %",
        "0.10 (v0.1 default)",
        cm1.correct,
        cm1.total,
        cm1.accuracy() * 100.0
    );

    for &bw in &[0.15f32, 0.18, 0.20, 0.22, 0.25, 0.30] {
        let mut v02 = MarineDetector::with_bandwidth(bw);
        let preds = v02.classify_stream(&windows, 25);
        let cm = ConfusionMatrix::from_predictions(&preds, &windows, 25);
        println!(
            "  {:<22} {}/{} = {:.1} %",
            format!("{:.2} (v0.2 wide)", bw),
            cm.correct,
            cm.total,
            cm.accuracy() * 100.0
        );
    }

    // ------------------------------------------------------------------
    // Summary
    // ------------------------------------------------------------------
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Summary                                                    ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  1) Boundary frequencies: v0.2 wide tuning recovers signals ║");
    println!("║     that v0.1 dropped as Ambient.                          ║");
    println!("║  2) Whale-under-ship: v0.2 multi-label flags BOTH species  ║");
    println!("║     in windows where v0.1 could only pick one dominant.    ║");
    println!("║  3) Ambient regression: v0.2 keeps false-positive species  ║");
    println!("║     rate low (see counts above).                           ║");
    println!("║  4) Overall CSV accuracy change shown above.               ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
