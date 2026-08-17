#!/usr/bin/env python3
"""
Preprocess MBARI MARS hydrophone recordings for CricketBrain.

Reads raw WAV/FLAC hydrophone audio, extracts the dominant frequency per
short-time FFT window in the 10-500 Hz baleen-whale band, and writes a
CSV suitable for the MarineDetector.

Output CSV columns:
    timestamp_ms   — sample index / (sample_rate / 1000)
    dominant_freq  — dominant frequency in the current FFT window (Hz)
    rms_db         — RMS amplitude of the window in dB re 1 µPa (approx)
    event_label    — ground-truth label (Ambient, FinWhale, BlueWhale, Ship, Humpback)

Usage:
    python preprocess.py --synthetic              # Generate sample CSV (no data needed)
    python preprocess.py --wav data/raw/mars.wav  # Process a real MARS recording

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

# MARS decimated rate for baleen-whale work; full rate is 256 kHz.
SAMPLE_RATE = 2000  # Hz

# Characteristic marine-acoustic frequencies (Hz)
FIN = 20.0
BLUE = 80.0
SHIP = 140.0
HUMP = 200.0

# Short-time FFT window
FFT_WINDOW = 512   # ~256 ms at 2 kHz
FFT_HOP = 256      # 50 % overlap → one window every 128 ms


def extract_frequencies_from_signal(signal, sample_rate, window_size, hop_size):
    """Extract dominant frequency + RMS per FFT window."""
    results = []
    n = len(signal)

    for start in range(0, n - window_size, hop_size):
        chunk = signal[start:start + window_size]
        windowed = chunk * np.hanning(window_size)
        fft_mag = np.abs(np.fft.rfft(windowed))
        freqs = np.fft.rfftfreq(window_size, d=1.0 / sample_rate)

        # Focus on the 10-500 Hz baleen-whale band
        mask = (freqs >= 10) & (freqs <= 500)
        if not mask.any():
            results.append((start / sample_rate * 1000, 0.0, -120.0))
            continue

        masked_mag = fft_mag[mask]
        masked_freqs = freqs[mask]
        peak_idx = np.argmax(masked_mag)
        dom_freq = masked_freqs[peak_idx]

        rms = np.sqrt(np.mean(chunk ** 2))
        rms_db = 20.0 * np.log10(max(rms, 1e-10))

        results.append((start / sample_rate * 1000, dom_freq, rms_db))

    return results


def process_wav_file(wav_path, event_label, output_dir):
    """Process a single .wav/.flac hydrophone recording."""
    try:
        import soundfile as sf
    except ImportError:
        print("ERROR: soundfile not installed. Run: pip install -r requirements.txt")
        sys.exit(1)

    signal, sr = sf.read(str(wav_path))
    if signal.ndim > 1:
        signal = signal.mean(axis=1)
    if sr != SAMPLE_RATE:
        # Simple decimation (lossy but adequate for this demo)
        factor = max(1, sr // SAMPLE_RATE)
        signal = signal[::factor]

    results = extract_frequencies_from_signal(signal, SAMPLE_RATE, FFT_WINDOW, FFT_HOP)

    os.makedirs(output_dir, exist_ok=True)
    csv_path = output_dir / f"{wav_path.stem}.csv"
    with open(csv_path, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['timestamp_ms', 'dominant_freq', 'rms_db', 'event_label'])
        for ts, freq, rms_db in results:
            writer.writerow([f'{ts:.2f}', f'{freq:.2f}', f'{rms_db:.2f}', event_label])
    return len(results)


def generate_synthetic(output_path, n_per_class=40):
    """Generate a deterministic synthetic sample CSV without real data."""
    os.makedirs(output_path.parent, exist_ok=True)
    rng = np.random.default_rng(42)

    rows = []
    t = 0.0
    dt = FFT_HOP / SAMPLE_RATE * 1000  # ms per window (~128 ms)

    # --- Ambient ---
    for _ in range(n_per_class):
        if rng.random() < 0.05:
            f = rng.uniform(250.0, 400.0)  # occasional biological noise
        else:
            f = 0.0
        rms = rng.uniform(-120.0, -95.0)
        rows.append((f'{t:.2f}', f'{f:.2f}', f'{rms:.2f}', 'Ambient'))
        t += dt

    # --- Fin whale 20 Hz ---
    for _ in range(n_per_class):
        f = FIN + rng.normal(0, 0.6)
        rms = rng.uniform(-90.0, -70.0)
        rows.append((f'{t:.2f}', f'{f:.2f}', f'{rms:.2f}', 'FinWhale'))
        t += dt

    # --- Blue whale A-call 80 Hz ---
    for _ in range(n_per_class):
        f = BLUE + rng.normal(0, 2.0)
        rms = rng.uniform(-85.0, -65.0)
        rows.append((f'{t:.2f}', f'{f:.2f}', f'{rms:.2f}', 'BlueWhale'))
        t += dt

    # --- Ship cavitation 140 Hz ---
    for _ in range(n_per_class):
        f = SHIP + rng.normal(0, 4.0)
        rms = rng.uniform(-75.0, -55.0)
        rows.append((f'{t:.2f}', f'{f:.2f}', f'{rms:.2f}', 'Ship'))
        t += dt

    # --- Humpback song 200 Hz ---
    for _ in range(n_per_class):
        f = HUMP + rng.normal(0, 5.0)
        rms = rng.uniform(-80.0, -60.0)
        rows.append((f'{t:.2f}', f'{f:.2f}', f'{rms:.2f}', 'Humpback'))
        t += dt

    with open(output_path, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['timestamp_ms', 'dominant_freq', 'rms_db', 'event_label'])
        writer.writerows(rows)

    print(f"Synthetic sample written: {output_path} ({len(rows)} windows)")


def main():
    parser = argparse.ArgumentParser(description="Preprocess MARS hydrophone data")
    parser.add_argument('--wav', type=str, help="Process a single .wav/.flac recording")
    parser.add_argument('--label', type=str, default='Unknown',
                        help="Event label for --wav mode "
                             "(Ambient, FinWhale, BlueWhale, Ship, Humpback)")
    parser.add_argument('--synthetic', action='store_true',
                        help="Generate synthetic sample CSV")
    args = parser.parse_args()

    if args.synthetic:
        generate_synthetic(DATA_PROC / "sample_marine.csv")
        return

    if args.wav:
        wav_path = Path(args.wav)
        if not wav_path.exists():
            print(f"ERROR: {wav_path} not found")
            sys.exit(1)
        n = process_wav_file(wav_path, args.label, DATA_PROC)
        print(f"Processed {wav_path.name}: {n} frequency windows")
        return

    print("Usage:")
    print("  python preprocess.py --synthetic")
    print("  python preprocess.py --wav data/raw/mars.wav --label FinWhale")


if __name__ == "__main__":
    main()
