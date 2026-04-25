#!/usr/bin/env python3
"""
Generate visualization plots for grid event detection results.
Outputs to docs/: event_timeline.png, confusion_matrix.png, confidence_dist.png
Date: 2026-04-24
"""

import re
import subprocess
import sys
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

PROJECT_ROOT = Path(__file__).resolve().parent.parent
DOCS_DIR = PROJECT_ROOT / "docs"
CSV_PATH = PROJECT_ROOT / "data" / "processed" / "sample_grid.csv"
CARGO_TOML = PROJECT_ROOT / "Cargo.toml"

COLORS = {
    "Outage": "#94a3b8",
    "Nominal (50 Hz)": "#00d4aa",
    "2nd Harmonic (100 Hz)": "#f59e0b",
    "3rd Harmonic (150 Hz)": "#ef4444",
    "4th Harmonic (200 Hz)": "#3b82f6",
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
    out = []
    pat = re.compile(r"\[\s*(\d+)\]\s+(.+?)\s+\|\s+Conf=([\d.]+)\s+\|\s+Step=(\d+)")
    for line in result.stdout.split("\n"):
        m = pat.search(line)
        if m:
            out.append({"idx": int(m.group(1)), "cls": m.group(2).strip(),
                        "conf": float(m.group(3)), "step": int(m.group(4))})
    return out


def plot_timeline(results):
    fig, ax = plt.subplots(figsize=(12, 3.5), facecolor=BG)
    ax.set_facecolor(CARD)
    for r in results:
        c = COLORS.get(r["cls"], MUTED)
        ax.barh(0, 1, left=r["idx"] - 1, color=c, height=0.6, edgecolor="none")
    ax.set_xlabel("Detection Window", color=TEXT)
    ax.set_yticks([])
    ax.set_title("Power-Grid Event Timeline", color=TEXT, fontsize=13, fontweight="bold")
    ax.tick_params(colors=MUTED)
    for s in ax.spines.values():
        s.set_color(MUTED); s.set_alpha(0.3)
    for cls, c in COLORS.items():
        ax.barh([], [], color=c, label=cls)
    ax.legend(loc="upper right", fontsize=8, facecolor=CARD, edgecolor=MUTED, labelcolor=TEXT)
    fig.tight_layout()
    p = DOCS_DIR / "event_timeline.png"
    fig.savefig(p, dpi=150, facecolor=BG)
    plt.close(fig)
    print(f"  Saved: {p}")


def plot_confusion(results):
    classes = list(COLORS.keys())
    short = ["Outage", "Nominal", "H2", "H3", "H4"]
    n = len(classes)
    matrix = np.zeros((n, n), dtype=int)
    for r in results:
        win = r["step"] // 25
        if win < 40: ti = 0
        elif win < 80: ti = 1
        elif win < 120: ti = 2
        elif win < 160: ti = 3
        else: ti = 4
        pi = classes.index(r["cls"]) if r["cls"] in classes else 0
        matrix[ti, pi] += 1

    fig, ax = plt.subplots(figsize=(6.5, 5.5), facecolor=BG)
    ax.set_facecolor(CARD)
    im = ax.imshow(matrix, cmap="YlOrBr", aspect="auto")
    ax.set_xticks(range(n)); ax.set_yticks(range(n))
    ax.set_xticklabels(short, color=TEXT, fontsize=9, rotation=30, ha="right")
    ax.set_yticklabels(short, color=TEXT, fontsize=9)
    ax.set_xlabel("Predicted", color=TEXT, fontsize=11)
    ax.set_ylabel("True", color=TEXT, fontsize=11)
    ax.set_title("Power-Grid Confusion Matrix", color=TEXT, fontsize=13, fontweight="bold")
    for i in range(n):
        for j in range(n):
            c = "white" if matrix[i, j] > matrix.max() / 2 else TEXT
            ax.text(j, i, str(matrix[i, j]), ha="center", va="center", color=c, fontsize=12, fontweight="bold")
    fig.colorbar(im, ax=ax, shrink=0.8)
    fig.tight_layout()
    p = DOCS_DIR / "confusion_matrix.png"
    fig.savefig(p, dpi=150, facecolor=BG)
    plt.close(fig)
    print(f"  Saved: {p}")


def plot_confidence(results):
    fig, ax = plt.subplots(figsize=(8, 4), facecolor=BG)
    ax.set_facecolor(CARD)
    for cls, c in COLORS.items():
        confs = [r["conf"] for r in results if r["cls"] == cls]
        if confs:
            ax.hist(confs, bins=20, range=(0, 1), alpha=0.6, color=c,
                    label=f"{cls} (n={len(confs)})", edgecolor="none")
    ax.set_xlabel("Confidence", color=TEXT)
    ax.set_ylabel("Count", color=TEXT)
    ax.set_title("Confidence Distribution", color=TEXT, fontsize=13, fontweight="bold")
    ax.tick_params(colors=MUTED)
    for s in ax.spines.values():
        s.set_color(MUTED); s.set_alpha(0.3)
    ax.legend(fontsize=8, facecolor=CARD, edgecolor=MUTED, labelcolor=TEXT)
    fig.tight_layout()
    p = DOCS_DIR / "confidence_dist.png"
    fig.savefig(p, dpi=150, facecolor=BG)
    plt.close(fig)
    print(f"  Saved: {p}")


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
