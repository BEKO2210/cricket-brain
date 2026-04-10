#!/usr/bin/env python3
"""
Generate visualization plots for bearing fault detection results.
Outputs to docs/: fault_timeline.png, confusion_matrix.png, confidence_dist.png
Date: 2026-04-10
"""

import subprocess
import sys
import re
from pathlib import Path
from collections import defaultdict

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

PROJECT_ROOT = Path(__file__).resolve().parent.parent
DOCS_DIR = PROJECT_ROOT / "docs"
CSV_PATH = PROJECT_ROOT / "data" / "processed" / "sample_bearing.csv"
CARGO_TOML = PROJECT_ROOT / "Cargo.toml"

COLORS = {
    "Normal": "#00d4aa",
    "Outer Race (BPFO)": "#ef4444",
    "Inner Race (BPFI)": "#f59e0b",
    "Ball Defect (BSF)": "#3b82f6",
}
BG = "#0a0f1c"
CARD = "#1e293b"
TEXT = "#f1f5f9"
MUTED = "#94a3b8"


def run_detector():
    result = subprocess.run(
        ["cargo", "run", "--release", "--manifest-path", str(CARGO_TOML),
         "--", "--csv", str(CSV_PATH)],
        capture_output=True, text=True, timeout=60
    )
    if result.returncode != 0:
        print("ERROR:", result.stderr, file=sys.stderr)
        sys.exit(1)
    results = []
    pattern = re.compile(r'\[\s*(\d+)\]\s+(.+?)\s+\|\s+Conf=([\d.]+)\s+\|\s+Step=(\d+)')
    for line in result.stdout.split('\n'):
        m = pattern.search(line)
        if m:
            results.append({
                "idx": int(m.group(1)),
                "cls": m.group(2).strip(),
                "conf": float(m.group(3)),
                "step": int(m.group(4)),
            })
    return results


def plot_timeline(results):
    fig, ax = plt.subplots(figsize=(12, 3.5), facecolor=BG)
    ax.set_facecolor(CARD)

    for r in results:
        color = COLORS.get(r["cls"], MUTED)
        ax.barh(0, 1, left=r["idx"] - 1, color=color, height=0.6, edgecolor='none')

    ax.set_xlabel("Detection Window", color=TEXT, fontsize=10)
    ax.set_yticks([])
    ax.set_title("Bearing Fault Timeline", color=TEXT, fontsize=13, fontweight='bold')
    ax.tick_params(colors=MUTED)
    for s in ax.spines.values():
        s.set_color(MUTED); s.set_alpha(0.3)

    for cls, color in COLORS.items():
        ax.barh([], [], color=color, label=cls)
    ax.legend(loc='upper right', fontsize=8, facecolor=CARD, edgecolor=MUTED, labelcolor=TEXT)

    fig.tight_layout()
    path = DOCS_DIR / "fault_timeline.png"
    fig.savefig(path, dpi=150, facecolor=BG)
    plt.close(fig)
    print(f"  Saved: {path}")


def plot_confusion(results):
    classes = ["Normal", "Outer Race (BPFO)", "Inner Race (BPFI)", "Ball Defect (BSF)"]
    short = ["Normal", "Outer", "Inner", "Ball"]
    n = len(classes)
    matrix = np.zeros((n, n), dtype=int)

    for r in results:
        win = r["step"] // 25
        if win < 50: ti = 0
        elif win < 100: ti = 1
        elif win < 150: ti = 2
        else: ti = 3

        pi = classes.index(r["cls"]) if r["cls"] in classes else 0
        matrix[ti, pi] += 1

    fig, ax = plt.subplots(figsize=(6, 5), facecolor=BG)
    ax.set_facecolor(CARD)
    im = ax.imshow(matrix, cmap='YlOrRd', aspect='auto')
    ax.set_xticks(range(n)); ax.set_yticks(range(n))
    ax.set_xticklabels(short, color=TEXT, fontsize=9, rotation=30, ha='right')
    ax.set_yticklabels(short, color=TEXT, fontsize=9)
    ax.set_xlabel("Predicted", color=TEXT, fontsize=11)
    ax.set_ylabel("True", color=TEXT, fontsize=11)
    ax.set_title("Confusion Matrix", color=TEXT, fontsize=13, fontweight='bold')

    for i in range(n):
        for j in range(n):
            c = "white" if matrix[i, j] > matrix.max() / 2 else TEXT
            ax.text(j, i, str(matrix[i, j]), ha='center', va='center', color=c, fontsize=14, fontweight='bold')

    fig.colorbar(im, ax=ax, shrink=0.8)
    fig.tight_layout()
    path = DOCS_DIR / "confusion_matrix.png"
    fig.savefig(path, dpi=150, facecolor=BG)
    plt.close(fig)
    print(f"  Saved: {path}")


def plot_confidence(results):
    fig, ax = plt.subplots(figsize=(8, 4), facecolor=BG)
    ax.set_facecolor(CARD)

    for cls, color in COLORS.items():
        confs = [r["conf"] for r in results if r["cls"] == cls]
        if confs:
            ax.hist(confs, bins=20, range=(0, 1), alpha=0.6, color=color,
                    label=f"{cls} (n={len(confs)})", edgecolor='none')

    ax.set_xlabel("Confidence", color=TEXT, fontsize=10)
    ax.set_ylabel("Count", color=TEXT, fontsize=10)
    ax.set_title("Confidence Distribution", color=TEXT, fontsize=13, fontweight='bold')
    ax.tick_params(colors=MUTED)
    for s in ax.spines.values():
        s.set_color(MUTED); s.set_alpha(0.3)
    ax.legend(fontsize=8, facecolor=CARD, edgecolor=MUTED, labelcolor=TEXT)

    fig.tight_layout()
    path = DOCS_DIR / "confidence_dist.png"
    fig.savefig(path, dpi=150, facecolor=BG)
    plt.close(fig)
    print(f"  Saved: {path}")


def main():
    DOCS_DIR.mkdir(parents=True, exist_ok=True)
    print("Running detector and generating plots...\n")
    results = run_detector()
    if not results:
        print("ERROR: No results"); sys.exit(1)
    print(f"Parsed {len(results)} classifications.\n")
    plot_timeline(results)
    plot_confusion(results)
    plot_confidence(results)
    print(f"\nAll plots saved to {DOCS_DIR}/")


if __name__ == "__main__":
    main()
