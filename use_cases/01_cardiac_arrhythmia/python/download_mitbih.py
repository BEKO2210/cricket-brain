#!/usr/bin/env python3
"""
Download MIT-BIH Arrhythmia Database from PhysioNet.

Dataset: https://physionet.org/content/mitdb/1.0.0/
License: Open Data Commons Attribution v1.0
Citation: Moody & Mark (2001), Goldberger et al. (2000)

Usage:
    pip install -r requirements.txt
    python download_mitbih.py
"""

import os
import sys
from pathlib import Path

# MIT-BIH record numbers (48 records total)
RECORDS_100 = [100, 101, 102, 103, 104, 105, 106, 107, 108, 109,
               111, 112, 113, 114, 115, 116, 117, 118, 119]
RECORDS_200 = [200, 201, 202, 203, 205, 207, 208, 209, 210, 212,
               213, 214, 215, 217, 219, 220, 221, 222, 223, 228,
               230, 231, 232, 233, 234]
ALL_RECORDS = RECORDS_100 + RECORDS_200

DATA_DIR = Path(__file__).resolve().parent.parent / "data" / "raw"
DB_NAME = "mitdb"


def check_record_exists(record_num: int) -> bool:
    """Check if a record's files already exist locally."""
    base = DATA_DIR / DB_NAME / str(record_num)
    return base.with_suffix(".dat").exists() and base.with_suffix(".hea").exists()


def download_all():
    """Download all MIT-BIH records using wfdb."""
    try:
        import wfdb
    except ImportError:
        print("ERROR: wfdb not installed. Run: pip install -r requirements.txt")
        sys.exit(1)

    dest = str(DATA_DIR / DB_NAME)
    os.makedirs(dest, exist_ok=True)

    already = sum(1 for r in ALL_RECORDS if check_record_exists(r))
    if already == len(ALL_RECORDS):
        print(f"All {len(ALL_RECORDS)} records already downloaded in {dest}")
        return

    print(f"Downloading MIT-BIH Arrhythmia Database ({len(ALL_RECORDS)} records)")
    print(f"Destination: {dest}")
    print(f"Already present: {already}/{len(ALL_RECORDS)}")
    print()

    for i, rec in enumerate(ALL_RECORDS):
        if check_record_exists(rec):
            print(f"  [{i+1}/{len(ALL_RECORDS)}] Record {rec}: already exists, skipping")
            continue

        print(f"  [{i+1}/{len(ALL_RECORDS)}] Record {rec}: downloading...", end=" ")
        try:
            wfdb.dl_database(DB_NAME, dest, records=[str(rec)])
            print("OK")
        except Exception as e:
            print(f"FAILED: {e}")

    print(f"\nDone. Records in: {dest}")
    print("License: Open Data Commons Attribution v1.0")
    print("Citation: Goldberger et al. (2000), Moody & Mark (2001)")


if __name__ == "__main__":
    download_all()
