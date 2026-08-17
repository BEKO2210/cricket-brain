#!/usr/bin/env python3
"""
Preprocess MIT-BIH Arrhythmia Database for CricketBrain cardiac detector.

Reads raw wfdb records, extracts R-peak annotations, computes R-R intervals,
and maps them to frequency-domain input for CricketBrain.

Output CSV columns (v0.6 — 7-col format, backward-compat aware loader):
    timestamp_ms   — annotation sample index / (sample_rate / 1000)
    rr_interval_ms — R-R interval in milliseconds
    beat_type      — AAMI beat annotation (N, S, V, F, Q)
    bpm            — instantaneous heart rate (60000 / rr_ms)
    mapped_freq    — frequency for CricketBrain input
    record_id      — MIT-BIH record identifier ("100", "212", "synth_normal", ...)
    rhythm_label   — MIT-BIH rhythm annotation in effect at this beat
                     ((AFIB, (N, (B, (T, (SBR, (SVTA, (VT, (AFL, (NOD, ...).
                     Empty string when the record has no rhythm annotations.

Train/Test split:
    Records 100-119: Training set
    Records 200-234: Test set

Usage:
    python preprocess.py                    # Process all downloaded records
    python preprocess.py --record 100       # Process single record
    python preprocess.py --synthetic        # Generate synthetic sample only

License: Script is AGPL-3.0. Dataset is ODC-By (see SOURCES.md).
"""

import argparse
import csv
import os
import sys
from pathlib import Path

DATA_RAW = Path(__file__).resolve().parent.parent / "data" / "raw" / "mitdb"
DATA_PROC = Path(__file__).resolve().parent.parent / "data" / "processed"

# AAMI beat type mapping (wfdb symbol → AAMI class)
AAMI_MAP = {
    "N": "N", "L": "N", "R": "N", "e": "N", "j": "N",  # Normal
    "A": "S", "a": "S", "J": "S", "S": "S",              # Supraventricular
    "V": "V", "E": "V",                                    # Ventricular
    "F": "F",                                               # Fusion
    "/": "Q", "f": "Q", "Q": "Q",                         # Unknown/Paced
}

# Records split
TRAIN_RECORDS = [100, 101, 102, 103, 104, 105, 106, 107, 108, 109,
                 111, 112, 113, 114, 115, 116, 117, 118, 119]
TEST_RECORDS = [200, 201, 202, 203, 205, 207, 208, 209, 210, 212,
                213, 214, 215, 217, 219, 220, 221, 222, 223, 228,
                230, 231, 232, 233, 234]

SAMPLE_RATE = 360  # MIT-BIH sampling rate (Hz)


def process_record(record_num: int, output_dir: Path) -> int:
    """Process a single MIT-BIH record. Returns number of beats written."""
    try:
        import wfdb
    except ImportError:
        print("ERROR: wfdb not installed. Run: pip install -r requirements.txt")
        sys.exit(1)

    record_path = str(DATA_RAW / str(record_num))

    if not Path(record_path + ".dat").exists():
        print(f"  Record {record_num}: not downloaded, skipping")
        return 0

    # Read annotations
    ann = wfdb.rdann(record_path, "atr")
    samples = ann.sample   # R-peak sample indices
    symbols = ann.symbol   # Beat type symbols
    aux_notes = ann.aux_note  # Non-empty for rhythm-change rows ('+')

    # Build a sorted list of rhythm-change events: (sample, rhythm_label).
    # The rhythm "in effect" at any later beat is the most recent one.
    rhythm_events = []
    for i, sym in enumerate(symbols):
        if sym == "+":
            note = aux_notes[i].rstrip("\x00").strip() if aux_notes[i] else ""
            if note.startswith("("):
                rhythm_events.append((samples[i], note))
    # Sort just to be safe (annotations are usually time-ordered already).
    rhythm_events.sort(key=lambda x: x[0])

    def rhythm_at(sample_idx: int) -> str:
        """Most recent rhythm label whose sample <= given sample."""
        if not rhythm_events:
            return ""
        # Binary search for the rightmost event with sample <= sample_idx.
        lo, hi = 0, len(rhythm_events)
        while lo < hi:
            mid = (lo + hi) // 2
            if rhythm_events[mid][0] <= sample_idx:
                lo = mid + 1
            else:
                hi = mid
        if lo == 0:
            return ""  # before first rhythm event
        return rhythm_events[lo - 1][1]

    # Compute R-R intervals
    rows = []
    for i in range(1, len(samples)):
        sym = symbols[i]
        aami = AAMI_MAP.get(sym)
        if aami is None:
            continue  # Skip non-beat annotations (+, ~, |, etc.)

        rr_samples = samples[i] - samples[i - 1]
        rr_ms = rr_samples / SAMPLE_RATE * 1000.0
        timestamp_ms = samples[i] / SAMPLE_RATE * 1000.0

        if rr_ms < 200 or rr_ms > 3000:
            continue  # Physiologically implausible

        bpm = 60000.0 / rr_ms
        # Map BPM to frequency: 40-200 BPM → 2000-5000 Hz range
        mapped_freq = 2000.0 + (bpm - 40.0) * (3000.0 / 160.0)
        mapped_freq = max(1000.0, min(8000.0, mapped_freq))

        rhythm_label = rhythm_at(samples[i])

        rows.append({
            "timestamp_ms": f"{timestamp_ms:.1f}",
            "rr_interval_ms": f"{rr_ms:.1f}",
            "beat_type": aami,
            "bpm": f"{bpm:.1f}",
            "mapped_freq": f"{mapped_freq:.1f}",
            "record_id": str(record_num),
            "rhythm_label": rhythm_label,
        })

    # Write CSV
    os.makedirs(output_dir, exist_ok=True)
    csv_path = output_dir / f"record_{record_num}.csv"

    with open(csv_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["timestamp_ms", "rr_interval_ms",
                                                "beat_type", "bpm", "mapped_freq",
                                                "record_id", "rhythm_label"])
        writer.writeheader()
        writer.writerows(rows)

    return len(rows)


def generate_synthetic(output_path: Path, n_per_class: int = 50):
    """Generate synthetic sample CSV without needing real data download.

    The synthetic sample uses the v0.3 6-column format with explicit
    `record_id = "synth_<class>"`. The string prefix `synth_` is what
    the `cardiac_mitbih` bench uses to refuse to publish "validated"
    numbers — synthetic record IDs are visible to the loader.
    """
    os.makedirs(output_path.parent, exist_ok=True)

    rows = []
    t = 0.0

    blocks = [
        # (rr_ms, record_id)
        (820.0, "synth_normal"),     # Normal sinus ~73 BPM
        (400.0, "synth_tachy"),      # Tachycardia ~150 BPM
        (1500.0, "synth_brady"),     # Bradycardia ~40 BPM
    ]
    for rr, rid in blocks:
        for _ in range(n_per_class):
            bpm = 60000.0 / rr
            freq = 2000.0 + (bpm - 40.0) * (3000.0 / 160.0)
            rows.append({
                "timestamp_ms": f"{t:.1f}",
                "rr_interval_ms": f"{rr:.1f}",
                "beat_type": "N",
                "bpm": f"{bpm:.1f}",
                "mapped_freq": f"{freq:.1f}",
                "record_id": rid,
                "rhythm_label": "",
            })
            t += rr

    with open(output_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["timestamp_ms", "rr_interval_ms",
                                                "beat_type", "bpm", "mapped_freq",
                                                "record_id", "rhythm_label"])
        writer.writeheader()
        writer.writerows(rows)

    print(f"Synthetic sample written: {output_path} ({len(rows)} rows)")


def main():
    parser = argparse.ArgumentParser(description="Preprocess MIT-BIH for CricketBrain")
    parser.add_argument("--record", type=int, help="Process single record number")
    parser.add_argument("--synthetic", action="store_true",
                        help="Generate synthetic sample CSV only (no download needed)")
    args = parser.parse_args()

    if args.synthetic:
        generate_synthetic(DATA_PROC / "sample_record.csv")
        return

    if args.record:
        n = process_record(args.record, DATA_PROC / "train"
                           if args.record < 200 else DATA_PROC / "test")
        print(f"Record {args.record}: {n} beats")
        return

    # Process all downloaded records
    total = 0
    for rec in TRAIN_RECORDS:
        n = process_record(rec, DATA_PROC / "train")
        if n > 0:
            print(f"  Record {rec} (train): {n} beats")
            total += n

    for rec in TEST_RECORDS:
        n = process_record(rec, DATA_PROC / "test")
        if n > 0:
            print(f"  Record {rec} (test): {n} beats")
            total += n

    print(f"\nTotal: {total} beats processed")
    if total == 0:
        print("No records found. Run download_mitbih.py first, or use --synthetic")


if __name__ == "__main__":
    main()
