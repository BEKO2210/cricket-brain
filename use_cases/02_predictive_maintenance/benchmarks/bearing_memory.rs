// SPDX-License-Identifier: AGPL-3.0-only
//! Memory footprint benchmark for bearing fault detector.
//! Date: 2026-04-10

use cricket_brain_bearings::detector::BearingDetector;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Bearing Memory Benchmark                                  ║");
    println!("║  Date: 2026-04-10                                          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let det = BearingDetector::new();
    let ram = det.memory_usage_bytes();
    let neurons = det.total_neurons();
    let struct_size = std::mem::size_of::<BearingDetector>();

    println!("  ResonatorBank RAM:        {} bytes", ram);
    println!("  Neurons:                  {} (4 channels x 5)", neurons);
    println!("  Bytes per neuron:         {:.1}", ram as f32 / neurons as f32);
    println!("  BearingDetector struct:   {} bytes (stack)", struct_size);
    println!("  Total estimated:          {} bytes", struct_size + ram);
    println!();
    println!("  Comparison:");
    println!("  - CricketBrain single (5N):  928 bytes");
    println!("  - This detector (20N):       {} bytes ({:.1}x)", ram, ram as f32 / 928.0);
    println!("  - ATtiny85 SRAM:             512 bytes — does NOT fit");
    println!("  - Arduino Uno SRAM:          2048 bytes — {}", if ram < 2048 { "FITS" } else { "NO" });
    println!("  - STM32F0 SRAM:              4096 bytes — FITS");
    println!("  - ESP32 SRAM:                520K bytes — FITS easily");
}
