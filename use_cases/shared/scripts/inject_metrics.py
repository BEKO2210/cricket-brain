#!/usr/bin/env python3
"""
inject_metrics.py — Single source of truth for CricketBrain metrics.

Reads use_cases/shared/metrics.json and replaces all {{key.subkey}} placeholders
in .md and .html files under use_cases/.

Usage:
    python use_cases/shared/scripts/inject_metrics.py            # apply changes
    python use_cases/shared/scripts/inject_metrics.py --dry-run   # preview only
"""

import json
import re
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
METRICS_FILE = SCRIPT_DIR.parent / "metrics.json"
USE_CASES_ROOT = SCRIPT_DIR.parent.parent  # use_cases/


def load_metrics() -> dict:
    """Load and flatten metrics.json into dot-notation keys."""
    with open(METRICS_FILE, "r", encoding="utf-8") as f:
        raw = json.load(f)

    flat = {}
    for section_key, section_val in raw.items():
        if isinstance(section_val, dict):
            for k, v in section_val.items():
                flat[f"{section_key}.{k}"] = v
        else:
            flat[section_key] = section_val
    return flat


def format_value(val) -> str:
    """Convert a metric value to its display string."""
    if val is None:
        return "TBD"
    if isinstance(val, bool):
        return "Yes" if val else "No"
    if isinstance(val, float):
        # Avoid trailing zeros: 0.175 stays 0.175, not 0.17500
        return f"{val:g}"
    return str(val)


def inject_file(filepath: Path, metrics: dict, dry_run: bool) -> list:
    """Replace all {{key}} placeholders in a single file.

    Returns a list of (placeholder, value) tuples that were replaced.
    """
    text = filepath.read_text(encoding="utf-8")
    replacements = []

    def replacer(match):
        key = match.group(1).strip()
        if key in metrics:
            val = format_value(metrics[key])
            replacements.append((key, val))
            return val
        # Also try use-case specific lookup:
        # If file is in 01_cardiac_arrhythmia/ and key is "uc.market_size_usd_bn",
        # resolve "uc" to the use-case section from metrics.json.
        if key.startswith("uc."):
            uc_dir = filepath.relative_to(USE_CASES_ROOT).parts[0]
            resolved_key = f"{uc_dir}.{key[3:]}"
            if resolved_key in metrics:
                val = format_value(metrics[resolved_key])
                replacements.append((key, val))
                return val
        # Unknown placeholder — leave as-is
        return match.group(0)

    new_text = re.sub(r"\{\{(.+?)\}\}", replacer, text)

    if replacements and not dry_run:
        filepath.write_text(new_text, encoding="utf-8")

    return replacements


def main():
    dry_run = "--dry-run" in sys.argv

    if not METRICS_FILE.exists():
        print(f"ERROR: {METRICS_FILE} not found.")
        sys.exit(1)

    metrics = load_metrics()
    print(f"Loaded {len(metrics)} metrics from {METRICS_FILE.name}")
    if dry_run:
        print("DRY RUN — no files will be modified.\n")

    # Find all .md and .html files under use_cases/
    target_files = sorted(
        list(USE_CASES_ROOT.rglob("*.md"))
        + list(USE_CASES_ROOT.rglob("*.html"))
    )

    total_files_changed = 0
    total_replacements = 0

    for fp in target_files:
        # Skip metrics.json itself and this script
        if fp.suffix not in (".md", ".html"):
            continue

        replacements = inject_file(fp, metrics, dry_run)
        if replacements:
            total_files_changed += 1
            total_replacements += len(replacements)
            rel = fp.relative_to(USE_CASES_ROOT)
            action = "would update" if dry_run else "updated"
            print(f"  {action}: {rel}")
            for key, val in replacements:
                print(f"    {{{{ {key} }}}} -> {val}")

    print(f"\n{'Would change' if dry_run else 'Changed'}: "
          f"{total_files_changed} file(s), {total_replacements} replacement(s)")


if __name__ == "__main__":
    main()
