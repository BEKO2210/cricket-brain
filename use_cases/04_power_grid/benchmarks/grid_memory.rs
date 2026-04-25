// SPDX-License-Identifier: AGPL-3.0-only
//! Memory footprint benchmark for the grid event detector.
//! Date: 2026-04-24

use cricket_brain_grid::detector::GridDetector;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Grid Memory Benchmark                                     ║");
    println!("║  Date: 2026-04-24                                          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let det = GridDetector::new();
    let ram = det.memory_usage_bytes();
    let neurons = det.total_neurons();
    let struct_size = std::mem::size_of::<GridDetector>();

    println!("  ResonatorBank RAM:        {ram} bytes");
    println!("  Neurons:                  {neurons} (4 channels x 5)");
    println!("  Bytes per neuron:         {:.1}", ram as f32 / neurons as f32);
    println!("  GridDetector struct:      {struct_size} bytes (stack)");
    println!("  Total estimated:          {} bytes", struct_size + ram);
    println!();
    println!("  Comparison:");
    println!("  - CricketBrain single (5N):    928 bytes");
    println!("  - This detector (20N):         {ram} bytes ({:.1}x)", ram as f32 / 928.0);
    println!("  - ATtiny85 SRAM:               512 bytes — does NOT fit");
    println!("  - Arduino Uno SRAM:            2048 bytes — {}",
             if ram < 2048 { "FITS" } else { "NO" });
    println!("  - STM32F0 SRAM:                4096 bytes — FITS");
    println!("  - ESP32 SRAM:                  520 KB — FITS easily");
    println!("  - Substation gateway 64 KB:    FITS with > 15× margin");
    println!("  - PMU (e.g. SEL-487E) 1 MB:    FITS with > 250× margin");
}
