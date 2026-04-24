# CricketBrain Application: Cardiac Arrhythmia Pre-Screening

> **Status:** v0.1.0 | **CricketBrain v3.0.0** | **License:** AGPL-3.0 | **Date:** 2026-04-10

> **NOT A MEDICAL DEVICE.** This application is a research prototype for educational
> and experimental purposes only. Do not use for clinical diagnosis, patient monitoring,
> or any safety-critical decision-making without appropriate regulatory approval
> (FDA 510(k), CE MDR Class IIa, etc.). See [disclaimer](#medical-disclaimer) below.

---

## Overview

Continuous ECG monitoring on wearables requires detecting irregular heartbeat
patterns in real-time. CricketBrain's 5-neuron coincidence detection circuit
provides sub-microsecond rhythm classification in 928 bytes of RAM — no cloud,
no training data, no battery drain.

**Market Size:** $50B wearable health monitoring

---

## Benchmark Results (2026-04-10)

### Classification Performance

| Class | TP | FP | FN | Precision | Recall | F1 |
|-------|---:|---:|---:|----------:|-------:|---:|
| Normal Sinus | 50 | 0 | 5 | 1.000 | 0.909 | 0.952 |
| Tachycardia | 43 | 0 | 4 | 1.000 | 0.915 | 0.956 |
| Bradycardia | 43 | 0 | 2 | 1.000 | 0.956 | 0.977 |
| **Macro Average** | | | | **1.000** | **0.927** | **0.962** |

**Accuracy:** 92.5% (136/147) on 150 synthetic beats

### Key Metrics

| Metric | Value | Method |
|--------|-------|--------|
| Accuracy | 92.5% | Confusion matrix on sample_record.csv |
| d' (SDT) | 6.18 | Green & Swets (1966), 200 trials/class |
| Macro F1 | 0.962 | Precision/Recall per class |
| Latency | 0.126 µs/step | Release mode, avg over 3 rhythms |
| Throughput | 7.9M steps/sec | Single CPU thread |
| RAM | 928 bytes | `memory_usage_bytes()` |
| Detector total | 1,336 bytes | Struct (408B) + CricketBrain heap (928B) |
| Arduino fit | 65% | Of 2,048B Arduino Uno RAM |

### Noise Rejection — With ECG Preprocessor

| Noise Level | Without Preprocessor | With Preprocessor | Improvement |
|-------------|---------------------:|------------------:|------------:|
| 0% (clean) | 100.0% | 100.0% | — |
| 10% | 42.1% | **75.6%** | +33.5% |
| 20% | 16.7% | **84.4%** | +67.7% |
| 30% | 35.1% | **70.0%** | +34.9% |
| 50% | 82.6% | 63.6% | — |
| 70% | 8.6% | 1.9% | Both fail |

The `EcgPreprocessor` applies temporal consistency filtering: in-band signals must persist
for 3+ consecutive steps. Single-step noise spikes are rejected. Enable via
`CardiacDetector::with_preprocessor(true)`.

### Other Stress Tests

| Test | Result | Verdict |
|------|--------|---------|
| Extreme rates (30–250 BPM) | 10/10 correct | PASSES |
| Boundary ±1 BPM (59/61/99/101) | 4/4 correct | PASSES |
| Random RR (irregular) | 82% detected | PASSES |
| Rapid switching (3-beat) | 89% Irregular | Expected |

See [docs/limitations.md](docs/limitations.md) for detailed failure analysis.

---

## Quick Start

```bash
git clone https://github.com/BEKO2210/cricket-brain.git
cd cricket-brain/use_cases/01_cardiac_arrhythmia

# Synthetic demo (Normal, Tachycardia, Bradycardia, Mixed)
cargo run --release

# Classify CSV data with confusion matrix
cargo run --release -- --csv data/processed/sample_record.csv

# Run all 11 tests
cargo test
```

### Expected Output

```
=== CricketBrain Cardiac Arrhythmia Pre-Screening ===
=== NOT a medical device — research prototype only ===

--- Normal Sinus Rhythm (5 cycles, expected ~73 BPM) ---
  Beat 1: Normal Sinus | BPM=73 | Confidence=0.70
  Beat 2: Normal Sinus | BPM=73 | Confidence=0.75
  Beat 3: Normal Sinus | BPM=73 | Confidence=0.80
  Final: Normal Sinus (BPM=73, Conf=0.80)

--- Tachycardia (5 cycles, expected ~150 BPM) ---
  Beat 1: Tachycardia | BPM=150 | Confidence=0.70
  Final: Tachycardia (BPM=150, Conf=0.80)

--- Bradycardia (5 cycles, expected ~40 BPM) ---
  Beat 1: Bradycardia | BPM=40 | Confidence=0.70
  Final: Bradycardia (BPM=40, Conf=0.80)
```

---

## Architecture

```
  ECG Signal ──→ R-Peak Extraction ──→ RR Intervals ──→ Frequency Mapping
                                                              │
                                               CricketBrain (5N/6S, 928 bytes)
                                               Gaussian tuning at QRS (4500 Hz)
                                               Coincidence detection gate
                                                              │
                                                       Spike Output
                                                              │
                                                    RR Interval Tracker
                                                    (sliding window, 8 beats)
                                                              │
                                               ┌──────────────┼──────────────┐
                                           >100 BPM      60-100 BPM      <60 BPM
                                          Tachycardia   Normal Sinus   Bradycardia
                                                     CV > 0.3 → Irregular
```

### ECG Waveform Model

Each cardiac cycle is encoded as frequency segments:

| Wave | Frequency | Duration | Biological Role |
|------|-----------|----------|-----------------|
| P wave | 3100 Hz | 12 ms | Atrial depolarization |
| Gap | 0 Hz | 4 ms | PR interval |
| QRS | 4500 Hz | 10 ms | Ventricular depolarization |
| Gap | 0 Hz | 4 ms | ST segment |
| T wave | 3400 Hz | 14 ms | Ventricular repolarization |
| Diastole | 0 Hz | variable | RR gap (determines BPM) |

The QRS complex is aligned to 4500 Hz — CricketBrain's carrier frequency — so
the coincidence detector fires on each ventricular beat.

---

## API

### Rust

```rust
use cricket_brain_cardiac::detector::{CardiacDetector, RhythmClass};
use cricket_brain_cardiac::ecg_signal;

// Create detector (928 bytes, adaptive sensitivity, privacy mode)
let mut detector = CardiacDetector::new();

// Feed frequency samples (1 ms timesteps)
for &freq in &ecg_signal::normal_sinus().to_frequency_stream(10) {
    if let Some(rhythm) = detector.step(freq) {
        println!("{} at {} BPM (conf={:.2})",
                 rhythm, detector.bpm_estimate(), detector.confidence());
    }
}

// Batch: classify a CSV file
let beats = ecg_signal::from_csv("data/processed/sample_record.csv");
let results = detector.classify_stream(&beats);
```

### Python (via cricket-brain PyO3 bindings)

```python
from cricket_brain import BrainConfig, Brain

config = BrainConfig()
config.adaptive_sensitivity = True
config.privacy_mode = True

brain = Brain(config)
for freq in ecg_frequencies:
    output = brain.step(freq)
    if output > 0.0:
        print(f"QRS spike detected: amplitude={output:.3f}")
```

### C/C++ (via cricket-brain FFI)

```c
#include "cricket_brain.h"

BrainHandle *h = NULL;
brain_new(&h, 5, 3000.0, 5000.0);

float output;
brain_step(h, 4500.0, &output);  // QRS frequency
if (output > 0.0f) {
    // Beat detected — update RR interval tracker
}

brain_free(h);
```

---

## Benchmarks

```bash
# Signal Detection Theory (d', AUC, Wilson CI)
cargo run --release --example cardiac_sdt

# Latency & throughput per rhythm type
cargo run --release --example cardiac_latency

# Memory footprint verification
cargo run --release --example cardiac_memory

# Adversarial stress test (noise, extreme rates, boundary)
cargo run --release --example cardiac_stress

# Criterion microbenchmark
cargo bench
```

---

## How It Compares (2026-04-24)

Short version — full breakdown with citations in
[docs/competitive_analysis.md](docs/competitive_analysis.md):

| System | RAM | Latency | Avg power @ 1 Hz | Training data |
|--------|----:|--------:|-----------------:|---------------|
| **CricketBrain UC01** | **928 B** | 0.126 µs/step | **< 1 µW compute** | **Zero** |
| Pan-Tompkins (QRS + rule-based) | < 1 KB | ~1-10 ms/beat | ~15 µW | Zero |
| Tiny MF-CNN (Nuzzo 2023) | ~4-8 KB | < 1 ms | ~500 µW | MIT-BIH inter-patient |
| Stanford DNN (Hannun 2019) | GPU class | 30 s window | N/A | 91 k ECGs, 53 k patients |
| Apple Watch AFib | proprietary | ~30-60 s | ~3 mW | Millions of users |

CricketBrain's niche is **sub-mW rate-based triage** (Normal / Tachy /
Brady / Irregular) on implantable, patch, or earbud-class hardware
where no CNN can live. For morphology (AF, VT, AVB, ST-elevation) use
a Tiny CNN or a commercial wearable.

---

## Dataset

| Field | Value |
|-------|-------|
| Name | MIT-BIH Arrhythmia Database |
| License | Open Data Commons Attribution v1.0 |
| URL | https://physionet.org/content/mitdb/1.0.0/ |
| Records | 48 × 30 min two-channel ambulatory ECG |
| Sampling | 360 Hz, 11-bit, 10 mV range |
| Annotations | ~110,000 beat labels by 2+ cardiologists |
| Citation | Goldberger et al. (2000), Moody & Mark (2001) |

```bash
# Download (requires wfdb: pip install -r python/requirements.txt)
python python/download_mitbih.py

# Preprocess to CSV
python python/preprocess.py

# Generate synthetic sample only (no download needed)
python python/preprocess.py --synthetic
```

See [data/SOURCES.md](data/SOURCES.md) for detailed provenance.

---

## Project Structure

```
01_cardiac_arrhythmia/
├── Cargo.toml              # Standalone project (cricket-brain dependency)
├── README.md               # This file
├── CLAUDE.md               # Build plan and run status
├── data/
│   ├── raw/                # MIT-BIH files (not committed)
│   ├── processed/
│   │   └── sample_record.csv   # 150 synthetic beats
│   └── SOURCES.md          # Dataset provenance
├── src/
│   ├── lib.rs              # Module exports
│   ├── detector.rs         # CardiacDetector + ConfusionMatrix
│   ├── ecg_signal.rs       # Waveform generation + CSV I/O
│   └── main.rs             # Demo binary (--csv mode)
├── benchmarks/
│   ├── cardiac_sdt.rs      # d' and AUC (Green & Swets 1966)
│   ├── cardiac_latency.rs  # First-classification timing
│   ├── cardiac_memory.rs   # RAM footprint verification
│   └── cardiac_stress.rs   # Adversarial conditions
├── benches/
│   └── cardiac_bench.rs    # Criterion microbenchmarks
├── python/
│   ├── requirements.txt    # wfdb, pandas, numpy, matplotlib
│   ├── download_mitbih.py  # Dataset downloader
│   ├── preprocess.py       # R-peak extraction pipeline
│   ├── evaluate.py         # Confusion matrix + F1 scores
│   └── plot_results.py     # Visualization (3 PNG plots)
├── docs/
│   ├── results.md          # Complete benchmark results
│   ├── limitations.md      # Honest failure analysis
│   ├── bpm_timeline.png    # BPM over time visualization
│   ├── confusion_matrix.png
│   └── confidence_dist.png
└── website/                # (placeholder for future web demo)
```

---

## Medical Disclaimer

> **THIS SOFTWARE IS NOT A MEDICAL DEVICE.**
>
> It has **not** been validated for clinical use, has **not** received regulatory
> clearance (FDA, CE, TGA, or equivalent), and **must NOT** be used for:
>
> - Clinical diagnosis or treatment decisions
> - Patient monitoring in healthcare settings
> - Any safety-critical or life-sustaining purpose
> - Screening programs without independent clinical validation
>
> This is a **research prototype** demonstrating neuromorphic signal processing.
> The 92.5% accuracy is measured on **synthetic data only** — real-world
> performance will differ. See [docs/limitations.md](docs/limitations.md).
>
> **No liability** is accepted for any use of this software in medical contexts.

---

## License

- **Source Code:** AGPL-3.0-only ([LICENSE](../../LICENSE))
- **Commercial Use:** Requires paid license ([COMMERCIAL.md](../../COMMERCIAL.md))
- **Dataset (MIT-BIH):** Open Data Commons Attribution v1.0
- **Citation Required:** Goldberger et al. (2000), Moody & Mark (2001)

---

## References

- [CricketBrain Research Whitepaper](../../RESEARCH_WHITEPAPER.md)
- [USE_CASES.md — Cardiac Arrhythmia](../../USE_CASES.md#1-cardiac-arrhythmia-pre-screening-on-wearables)
- [Existing Demo](../../examples/sentinel_ecg_monitor.rs)
- [Website Demo](../../website/pages/cardiac.html)
- [API Reference](docs/api.md)
- [Limitations](docs/limitations.md)
- [Benchmark Results](docs/results.md)

## Metrics Source

All metrics sourced from [`use_cases/shared/metrics.json`](../shared/metrics.json).
Run `python use_cases/shared/scripts/inject_metrics.py` to update.
