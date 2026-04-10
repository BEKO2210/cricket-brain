#!/usr/bin/env python3
"""
Preprocess CWRU Bearing Data Center .mat files for CricketBrain.

Reads raw accelerometer signals, extracts dominant vibration frequencies
via short-time FFT, and outputs CSV suitable for the BearingDetector.

Output CSV columns:
    timestamp_ms   — sample index / (sample_rate / 1000)
    dominant_freq  — dominant frequency in the current FFT window (Hz)
    amplitude      — peak amplitude of the dominant frequency
    fault_label    — ground truth fault type (Normal, IR, OR, Ball)

Usage:
    python preprocess.py --synthetic            # Generate sample CSV (no .mat needed)
    python preprocess.py --mat data/raw/100.mat # Process real CWRU file

Date: 2026-04-10
"""

import argparse
import csv
import os
import sys
from pathlib import Path

import numpy as np

DATA_RAW = Path(__file__).resolve().parent.parent / "data" / "raw"
DATA_PROC = Path(__file__).resolve().parent.parent / "data" / "processed"

# CWRU sampling rate
SAMPLE_RATE = 12000  # Hz

# SKF 6205-2RS characteristic frequencies at 1797 RPM
BPFO = 107.36
BPFI = 162.19
BSF = 69.04
FTF = 14.83

# FFT window for frequency extraction
FFT_WINDOW = 512  # ~42 ms at 12 kHz
FFT_HOP = 256     # 50% overlap


def extract_frequencies_from_signal(signal, sample_rate, window_size, hop_size):
    """Extract dominant frequency per window using FFT."""
    results = []
    n = len(signal)

    for start in range(0, n - window_size, hop_size):
        chunk = signal[start:start + window_size]
        # Apply Hanning window
        windowed = chunk * np.hanning(window_size)
        # FFT
        fft_mag = np.abs(np.fft.rfft(windowed))
        freqs = np.fft.rfftfreq(window_size, d=1.0 / sample_rate)

        # Find dominant frequency (skip DC, focus on 10-500 Hz)
        mask = (freqs >= 10) & (freqs <= 500)
        if not mask.any():
            results.append((start / sample_rate * 1000, 0.0, 0.0))
            continue

        masked_mag = fft_mag[mask]
        masked_freqs = freqs[mask]
        peak_idx = np.argmax(masked_mag)
        dom_freq = masked_freqs[peak_idx]
        dom_amp = masked_mag[peak_idx] / window_size

        results.append((start / sample_rate * 1000, dom_freq, dom_amp))

    return results


def process_mat_file(mat_path, fault_label, output_dir):
    """Process a single .mat file from CWRU dataset."""
    try:
        from scipy.io import loadmat
    except ImportError:
        print("ERROR: scipy not installed. Run: pip install -r requirements.txt")
        sys.exit(1)

    data = loadmat(str(mat_path))
    # CWRU .mat files have keys like 'X097_DE_time' for drive-end accelerometer
    signal_key = None
    for key in data:
        if 'DE_time' in key:
            signal_key = key
            break
    if signal_key is None:
        # Try any numeric array
        for key in data:
            if isinstance(data[key], np.ndarray) and data[key].ndim >= 1:
                signal_key = key
                break

    if signal_key is None:
        print(f"  WARNING: No signal found in {mat_path}")
        return 0

    signal = data[signal_key].flatten().astype(np.float64)
    results = extract_frequencies_from_signal(signal, SAMPLE_RATE, FFT_WINDOW, FFT_HOP)

    os.makedirs(output_dir, exist_ok=True)
    csv_path = output_dir / f"{mat_path.stem}.csv"

    with open(csv_path, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['timestamp_ms', 'dominant_freq', 'amplitude', 'fault_label'])
        for ts, freq, amp in results:
            writer.writerow([f'{ts:.2f}', f'{freq:.2f}', f'{amp:.6f}', fault_label])

    return len(results)


def generate_synthetic(output_path, n_windows=200):
    """Generate synthetic sample CSV without real CWRU data."""
    os.makedirs(output_path.parent, exist_ok=True)
    rng = np.random.default_rng(42)

    rows = []
    t = 0.0
    dt = FFT_HOP / SAMPLE_RATE * 1000  # ms per window

    # 50 normal windows
    for _ in range(50):
        freq = rng.uniform(20, 50)  # Random low-frequency vibration
        amp = rng.uniform(0.001, 0.01)
        rows.append((f'{t:.2f}', f'{freq:.2f}', f'{amp:.6f}', 'Normal'))
        t += dt

    # 50 outer race fault windows (BPFO ~107 Hz)
    for _ in range(50):
        freq = BPFO + rng.normal(0, 3)
        amp = rng.uniform(0.05, 0.15)
        rows.append((f'{t:.2f}', f'{freq:.2f}', f'{amp:.6f}', 'OR'))
        t += dt

    # 50 inner race fault windows (BPFI ~162 Hz)
    for _ in range(50):
        freq = BPFI + rng.normal(0, 4)
        amp = rng.uniform(0.04, 0.12)
        rows.append((f'{t:.2f}', f'{freq:.2f}', f'{amp:.6f}', 'IR'))
        t += dt

    # 50 ball defect windows (BSF ~69 Hz)
    for _ in range(50):
        freq = BSF + rng.normal(0, 2)
        amp = rng.uniform(0.03, 0.10)
        rows.append((f'{t:.2f}', f'{freq:.2f}', f'{amp:.6f}', 'Ball'))
        t += dt

    with open(output_path, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['timestamp_ms', 'dominant_freq', 'amplitude', 'fault_label'])
        writer.writerows(rows)

    print(f"Synthetic sample written: {output_path} ({len(rows)} windows)")


def main():
    parser = argparse.ArgumentParser(description="Preprocess CWRU bearing data")
    parser.add_argument('--mat', type=str, help="Process a single .mat file")
    parser.add_argument('--fault', type=str, default='Unknown',
                        help="Fault label for --mat mode (Normal, IR, OR, Ball)")
    parser.add_argument('--synthetic', action='store_true',
                        help="Generate synthetic sample CSV")
    args = parser.parse_args()

    if args.synthetic:
        generate_synthetic(DATA_PROC / "sample_bearing.csv")
        return

    if args.mat:
        mat_path = Path(args.mat)
        if not mat_path.exists():
            print(f"ERROR: {mat_path} not found")
            sys.exit(1)
        n = process_mat_file(mat_path, args.fault, DATA_PROC)
        print(f"Processed {mat_path.name}: {n} frequency windows")
        return

    print("Usage:")
    print("  python preprocess.py --synthetic")
    print("  python preprocess.py --mat data/raw/100.mat --fault Normal")


if __name__ == "__main__":
    main()
