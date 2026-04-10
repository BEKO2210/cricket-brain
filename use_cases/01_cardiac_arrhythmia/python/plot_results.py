#!/usr/bin/env python3
"""
Generate visualization plots for cardiac arrhythmia detection results.

Outputs:
    docs/bpm_timeline.png     — BPM over time with classification colors
    docs/confusion_matrix.png — Heatmap of prediction vs truth
    docs/confidence_dist.png  — Confidence distribution per class

Date: 2026-04-10
"""

import subprocess
import sys
import re
from pathlib import Path
from collections import defaultdict

import matplotlib
matplotlib.use('Agg')  # Non-interactive backend
import matplotlib.pyplot as plt
import numpy as np

PROJECT_ROOT = Path(__file__).resolve().parent.parent
DOCS_DIR = PROJECT_ROOT / "docs"
CSV_PATH = PROJECT_ROOT / "data" / "processed" / "sample_record.csv"
CARGO_TOML = PROJECT_ROOT / "Cargo.toml"

# CricketBrain color palette (matching website)
COLORS = {
    "Normal Sinus": "#00d4aa",   # accent teal
    "Tachycardia": "#ef4444",    # danger red
    "Bradycardia": "#3b82f6",    # blue
    "Irregular": "#f59e0b",      # highlight amber
}
BG_COLOR = "#0a0f1c"
CARD_COLOR = "#1e293b"
TEXT_COLOR = "#f1f5f9"
MUTED_COLOR = "#94a3b8"


def run_detector():
    """Run Rust detector and parse output."""
    result = subprocess.run(
        ["cargo", "run", "--release", "--manifest-path", str(CARGO_TOML),
         "--", "--csv", str(CSV_PATH)],
        capture_output=True, text=True, timeout=60
    )
    if result.returncode != 0:
        print("ERROR:", result.stderr, file=sys.stderr)
        sys.exit(1)

    results = []
    pattern = re.compile(
        r'\[\s*(\d+)\]\s+(\w[\w\s]*?)\s+\|\s+BPM=([\d.]+)\s+\|\s+Conf=([\d.]+)\s+\|\s+Step=(\d+)'
    )
    for line in result.stdout.split('\n'):
        m = pattern.search(line)
        if m:
            results.append({
                "idx": int(m.group(1)),
                "cls": m.group(2).strip(),
                "bpm": float(m.group(3)),
                "conf": float(m.group(4)),
                "step": int(m.group(5)),
            })
    return results


def plot_bpm_timeline(results):
    """BPM over time colored by classification."""
    fig, ax = plt.subplots(figsize=(12, 4), facecolor=BG_COLOR)
    ax.set_facecolor(CARD_COLOR)

    steps = [r["step"] / 1000 for r in results]  # Convert to seconds
    bpms = [r["bpm"] for r in results]
    colors = [COLORS.get(r["cls"], MUTED_COLOR) for r in results]

    ax.scatter(steps, bpms, c=colors, s=20, alpha=0.8, edgecolors='none')

    # Reference lines
    ax.axhline(y=100, color=COLORS["Tachycardia"], linestyle='--', alpha=0.3, linewidth=1)
    ax.axhline(y=60, color=COLORS["Bradycardia"], linestyle='--', alpha=0.3, linewidth=1)
    ax.text(steps[-1] * 0.98, 102, ">100 BPM = Tachy", color=COLORS["Tachycardia"],
            fontsize=8, ha='right', alpha=0.6)
    ax.text(steps[-1] * 0.98, 55, "<60 BPM = Brady", color=COLORS["Bradycardia"],
            fontsize=8, ha='right', alpha=0.6)

    ax.set_xlabel("Time (seconds)", color=TEXT_COLOR, fontsize=10)
    ax.set_ylabel("BPM", color=TEXT_COLOR, fontsize=10)
    ax.set_title("CricketBrain Cardiac Detector — BPM Timeline", color=TEXT_COLOR, fontsize=13, fontweight='bold')
    ax.tick_params(colors=MUTED_COLOR)
    for spine in ax.spines.values():
        spine.set_color(MUTED_COLOR)
        spine.set_alpha(0.3)

    # Legend
    for cls, color in COLORS.items():
        ax.scatter([], [], c=color, s=30, label=cls)
    ax.legend(loc='upper right', fontsize=8, facecolor=CARD_COLOR, edgecolor=MUTED_COLOR,
              labelcolor=TEXT_COLOR)

    fig.tight_layout()
    path = DOCS_DIR / "bpm_timeline.png"
    fig.savefig(path, dpi=150, facecolor=BG_COLOR)
    plt.close(fig)
    print(f"  Saved: {path}")


def plot_confusion_matrix(results):
    """Confusion matrix heatmap."""
    classes = ["Normal Sinus", "Tachycardia", "Bradycardia"]
    n = len(classes)
    matrix = np.zeros((n, n), dtype=int)

    for r in results:
        bpm = r["bpm"]
        if bpm > 100:
            true_idx = 1
        elif bpm < 60:
            true_idx = 2
        else:
            true_idx = 0

        pred_cls = r["cls"]
        if pred_cls in classes:
            pred_idx = classes.index(pred_cls)
        else:
            continue  # Skip Irregular

        matrix[true_idx, pred_idx] += 1

    fig, ax = plt.subplots(figsize=(6, 5), facecolor=BG_COLOR)
    ax.set_facecolor(CARD_COLOR)

    im = ax.imshow(matrix, cmap='YlGn', aspect='auto')

    # Labels
    ax.set_xticks(range(n))
    ax.set_yticks(range(n))
    ax.set_xticklabels(classes, color=TEXT_COLOR, fontsize=9, rotation=30, ha='right')
    ax.set_yticklabels(classes, color=TEXT_COLOR, fontsize=9)
    ax.set_xlabel("Predicted", color=TEXT_COLOR, fontsize=11)
    ax.set_ylabel("True", color=TEXT_COLOR, fontsize=11)
    ax.set_title("Confusion Matrix", color=TEXT_COLOR, fontsize=13, fontweight='bold')

    # Annotate cells
    for i in range(n):
        for j in range(n):
            val = matrix[i, j]
            color = "white" if val > matrix.max() / 2 else TEXT_COLOR
            ax.text(j, i, str(val), ha='center', va='center', color=color, fontsize=14, fontweight='bold')

    fig.colorbar(im, ax=ax, shrink=0.8)
    fig.tight_layout()
    path = DOCS_DIR / "confusion_matrix.png"
    fig.savefig(path, dpi=150, facecolor=BG_COLOR)
    plt.close(fig)
    print(f"  Saved: {path}")


def plot_confidence_distribution(results):
    """Confidence distribution per classification class."""
    fig, ax = plt.subplots(figsize=(8, 4), facecolor=BG_COLOR)
    ax.set_facecolor(CARD_COLOR)

    for cls, color in COLORS.items():
        confs = [r["conf"] for r in results if r["cls"] == cls]
        if confs:
            ax.hist(confs, bins=20, range=(0, 1), alpha=0.6, color=color, label=f"{cls} (n={len(confs)})",
                    edgecolor='none')

    ax.set_xlabel("Confidence", color=TEXT_COLOR, fontsize=10)
    ax.set_ylabel("Count", color=TEXT_COLOR, fontsize=10)
    ax.set_title("Confidence Distribution by Classification", color=TEXT_COLOR, fontsize=13, fontweight='bold')
    ax.tick_params(colors=MUTED_COLOR)
    for spine in ax.spines.values():
        spine.set_color(MUTED_COLOR)
        spine.set_alpha(0.3)
    ax.legend(fontsize=8, facecolor=CARD_COLOR, edgecolor=MUTED_COLOR, labelcolor=TEXT_COLOR)

    fig.tight_layout()
    path = DOCS_DIR / "confidence_dist.png"
    fig.savefig(path, dpi=150, facecolor=BG_COLOR)
    plt.close(fig)
    print(f"  Saved: {path}")


def main():
    DOCS_DIR.mkdir(parents=True, exist_ok=True)

    print("Running detector and generating plots...\n")
    results = run_detector()
    if not results:
        print("ERROR: No results parsed")
        sys.exit(1)

    print(f"Parsed {len(results)} classifications.\n")
    plot_bpm_timeline(results)
    plot_confusion_matrix(results)
    plot_confidence_distribution(results)
    print(f"\nAll plots saved to {DOCS_DIR}/")


if __name__ == "__main__":
    main()
