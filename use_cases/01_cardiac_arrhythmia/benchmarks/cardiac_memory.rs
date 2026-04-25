// SPDX-License-Identifier: AGPL-3.0-only
//! Memory footprint benchmark for cardiac detector.
//!
//! Verifies that the detector's RAM usage matches the global metric (928 bytes).
//!
//! Date: 2026-04-10

use cricket_brain_cardiac::detector::CardiacDetector;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Cardiac Memory Benchmark                                  ║");
    println!("║  Date: 2026-04-10                                          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let det = CardiacDetector::new();
    let ram = det.memory_usage_bytes();

    println!("  CricketBrain core RAM:    {} bytes", ram);
    println!("  metrics.json expected:    928 bytes");
    println!(
        "  Match:                    {}",
        if ram == 928 { "YES" } else { "MISMATCH" }
    );
    println!();

    // Measure CardiacDetector struct overhead (approximate)
    let struct_size = std::mem::size_of::<CardiacDetector>();
    println!("  CardiacDetector struct:   {} bytes (stack)", struct_size);
    println!("  CricketBrain heap:        {} bytes", ram);
    println!("  Total estimated:          {} bytes", struct_size + ram);

    // Comparison
    println!("\n  For reference:");
    println!("  - Arduino Uno total RAM:  2048 bytes");
    println!(
        "  - Detector fits:          {} ({:.0}% of Arduino RAM)",
        if struct_size + ram < 2048 {
            "YES"
        } else {
            "NO"
        },
        (struct_size + ram) as f64 / 2048.0 * 100.0
    );
}
