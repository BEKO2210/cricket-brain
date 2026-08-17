#!/usr/bin/env python3
"""
Preprocess EPFL Smart-Grid PMU streams for CricketBrain.

Reads raw PMU CSV/HDF5 voltage waveforms, extracts the dominant
spectral component per short-time FFT window in the 30-300 Hz band,
and writes a CSV suitable for the GridDetector.

Output CSV columns:
    timestamp_ms   — sample index / (sample_rate / 1000)
    dominant_freq  — dominant frequency in the FFT window (Hz)
    thd_pct        — total harmonic distortion (%) over harmonics 2-5
    event_label    — ground-truth label (Outage, Nominal, SecondHarmonic,
                     ThirdHarmonic, FourthHarmonic)

Usage:
    python preprocess.py --synthetic
    python preprocess.py --pmu data/raw/pmu_segment.csv

Date: 2026-04-24
"""

import argparse
import csv
import os
import sys
from pathlib import Path

import numpy as np

DATA_RAW = Path(__file__).resolve().parent.parent / "data" / "raw"
DATA_PROC = Path(__file__).resolve().parent.parent / "data" / "processed"

# Typical PMU reporting rate (50 Hz grid, 50 frames/s) up to instrumentation
# bandwidth. The EPFL dataset publishes time-series at 50 kHz aux waveform
# capture; we decimate to 4 kHz for the 30-300 Hz band of interest.
SAMPLE_RATE = 4000  # Hz

# 50 Hz grid harmonics
FUND = 50.0
H2 = 100.0
H3 = 150.0
H4 = 200.0

FFT_WINDOW = 1024  # ~256 ms at 4 kHz → strong frequency resolution
FFT_HOP = 512


def extract_freq_thd(signal, sample_rate, window_size, hop_size):
    """Extract dominant freq + THD per FFT window."""
    results = []
    for start in range(0, len(signal) - window_size, hop_size):
        chunk = signal[start:start + window_size]
        windowed = chunk * np.hanning(window_size)
        fft_mag = np.abs(np.fft.rfft(windowed))
        freqs = np.fft.rfftfreq(window_size, d=1.0 / sample_rate)

        mask = (freqs >= 30) & (freqs <= 300)
        if not mask.any():
            results.append((start / sample_rate * 1000, 0.0, 0.0))
            continue

        masked_mag = fft_mag[mask]
        masked_freqs = freqs[mask]
        peak_idx = np.argmax(masked_mag)
        dom_freq = masked_freqs[peak_idx]

        # THD over harmonics 2..5 referenced to the fundamental peak
        fund_amp = max(np.max(fft_mag[(freqs > 45) & (freqs < 55)]) if (freqs > 45).any() else 1e-9, 1e-9)
        harmonic_sum = 0.0
        for k in (2, 3, 4, 5):
            target = FUND * k
            band = (freqs > target - 5) & (freqs < target + 5)
            if band.any():
                harmonic_sum += np.max(fft_mag[band]) ** 2
        thd_pct = 100.0 * np.sqrt(harmonic_sum) / fund_amp
        results.append((start / sample_rate * 1000, dom_freq, thd_pct))
    return results


def process_pmu_file(pmu_path, label, output_dir):
    try:
        import pandas as pd
    except ImportError:
        print("ERROR: pandas not installed. Run: pip install -r requirements.txt")
        sys.exit(1)
    df = pd.read_csv(str(pmu_path))
    if df.shape[1] < 1:
        print(f"  WARNING: empty file {pmu_path}")
        return 0
    signal = df.iloc[:, -1].to_numpy(dtype=np.float64)
    results = extract_freq_thd(signal, SAMPLE_RATE, FFT_WINDOW, FFT_HOP)
    os.makedirs(output_dir, exist_ok=True)
    csv_path = output_dir / f"{pmu_path.stem}.csv"
    with open(csv_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["timestamp_ms", "dominant_freq", "thd_pct", "event_label"])
        for ts, freq, thd in results:
            writer.writerow([f"{ts:.2f}", f"{freq:.2f}", f"{thd:.2f}", label])
    return len(results)


def generate_synthetic(output_path, n_per_class=40):
    """Generate the deterministic synthetic sample CSV (no real data needed)."""
    os.makedirs(output_path.parent, exist_ok=True)
    rng = np.random.default_rng(42)
    rows = []
    t = 0.0
    dt = FFT_HOP / SAMPLE_RATE * 1000  # ms per window

    # Outage
    for _ in range(n_per_class):
        f = rng.uniform(250, 400) if rng.random() < 0.02 else 0.0
        thd = rng.uniform(0.0, 0.5)
        rows.append((f"{t:.2f}", f"{f:.2f}", f"{thd:.2f}", "Outage"))
        t += dt

    # Nominal
    for _ in range(n_per_class):
        f = FUND + rng.normal(0, 0.1)
        thd = rng.uniform(0.5, 2.5)
        rows.append((f"{t:.2f}", f"{f:.2f}", f"{thd:.2f}", "Nominal"))
        t += dt

    # 2nd harmonic
    for _ in range(n_per_class):
        f = H2 + rng.normal(0, 1.5)
        thd = rng.uniform(8.0, 18.0)
        rows.append((f"{t:.2f}", f"{f:.2f}", f"{thd:.2f}", "SecondHarmonic"))
        t += dt

    # 3rd harmonic
    for _ in range(n_per_class):
        f = H3 + rng.normal(0, 2.0)
        thd = rng.uniform(10.0, 25.0)
        rows.append((f"{t:.2f}", f"{f:.2f}", f"{thd:.2f}", "ThirdHarmonic"))
        t += dt

    # 4th harmonic
    for _ in range(n_per_class):
        f = H4 + rng.normal(0, 2.5)
        thd = rng.uniform(6.0, 15.0)
        rows.append((f"{t:.2f}", f"{f:.2f}", f"{thd:.2f}", "FourthHarmonic"))
        t += dt

    with open(output_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["timestamp_ms", "dominant_freq", "thd_pct", "event_label"])
        writer.writerows(rows)
    print(f"Synthetic sample written: {output_path} ({len(rows)} windows)")


def main():
    parser = argparse.ArgumentParser(description="Preprocess PMU stream for grid triage")
    parser.add_argument("--pmu", type=str, help="Process a single PMU CSV (last column = signal)")
    parser.add_argument("--label", type=str, default="Unknown",
                        help="Event label for --pmu mode (Outage / Nominal / SecondHarmonic / ThirdHarmonic / FourthHarmonic)")
    parser.add_argument("--synthetic", action="store_true", help="Generate synthetic sample CSV")
    args = parser.parse_args()

    if args.synthetic:
        generate_synthetic(DATA_PROC / "sample_grid.csv")
        return
    if args.pmu:
        path = Path(args.pmu)
        if not path.exists():
            print(f"ERROR: {path} not found")
            sys.exit(1)
        n = process_pmu_file(path, args.label, DATA_PROC)
        print(f"Processed {path.name}: {n} frequency windows")
        return
    print("Usage:")
    print("  python preprocess.py --synthetic")
    print("  python preprocess.py --pmu data/raw/pmu_segment.csv --label ThirdHarmonic")


if __name__ == "__main__":
    main()
