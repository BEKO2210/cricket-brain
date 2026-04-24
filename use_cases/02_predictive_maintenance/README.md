# CricketBrain Application: Bearing Fault-Frequency Triage

> **Status:** v0.1.0 research prototype | **CricketBrain v3.0.0** | **License:** AGPL-3.0 | **Date:** 2026-04-10
>
> **Validation status:** Synthetic CWRU-frequency signals only. Real
> CWRU `.mat` accelerometer-file validation is the top-priority
> milestone for v0.2 — see [MASTER_PLAN.md](../MASTER_PLAN.md).

---

## Overview

Rotating machinery (pumps, turbines, motors) develops bearing faults
that produce characteristic defect frequencies (BPFO, BPFI, BSF, FTF).
CricketBrain's ResonatorBank classifies which defect frequency is
dominant with 4 parallel 5-neuron circuits in 3,712 bytes of RAM —
**frequency-pattern triage**, not full mechanical diagnosis.

**What it is:** A fault-frequency classifier suitable as the front-end
of a wireless, self-powered vibration tag.

**What it is NOT:** An envelope-analysis severity estimator or a
classical condition-monitoring system. See
[What this detector does NOT do](#what-this-detector-does-not-do).

### Market context

Predictive-maintenance vibration monitoring is a ~$15 B annual segment
(MarketsandMarkets, 2024 estimate). **Target MCU:** STM32F0+ ($2).

---

## Benchmark Results (2026-04-10, synthetic SKF 6205-2RS signals)

### Classification Performance

| Fault Type | Freq (Hz) | TP | FP | Precision | Recall | F1 |
|---|---:|---:|---:|---:|---:|---:|
| Normal | — | 24 | 1 | 0.960 | 1.000 | 0.980 |
| Outer Race (BPFO) | 107 | 24 | 3 | 0.889 | 0.960 | 0.923 |
| Inner Race (BPFI) | 162 | 22 | 3 | 0.880 | 0.880 | 0.880 |
| Ball Defect (BSF) | 69 | 23 | 0 | 1.000 | 0.885 | 0.939 |
| **Macro Average** | | | | **0.932** | **0.931** | **0.932** |

**Synthetic-window accuracy:** 93 / 100 = **93.0 %** | **d' (SDT) [†]:** 6.18

[†] d' uses log-linear correction for ceiling hit-rates and floor
false-alarm rates (hits clipped to `[0.5/n, 1 − 0.5/n]` before the
inverse-normal). Without the correction, perfect TPR = 1.000 / FPR = 0.000
would yield an undefined / infinite d'.

### Noise Resilience (synthetic random-spike transients only)

| Noise % | Accuracy |
|------:|---------:|
| 0–50 % | 100 % on synthetic random-spike noise |

The 50-step energy accumulation window averages short random
transients. These numbers do **not** characterise behaviour under real
accelerometer conditions (sustained broadband noise, harmonic
interference, bearing natural frequencies, motor electrical noise).
Real CWRU `.mat` validation is planned.

### Speed Compensation

| RPM | Without | With `set_rpm()` |
|----:|:-------:|:----------------:|
| 900 | WRONG | **CORRECT** |
| 1200 | WRONG | **CORRECT** |
| 1500 | WRONG | **CORRECT** |
| 1797 | Correct | Correct |
| 2400 | WRONG | **CORRECT** |

**6/6 RPM levels correct with compensation.** Requires tachometer signal.

### Performance

| Metric | Value |
|--------|-------|
| Latency | 0.13–0.26 µs/step |
| RAM | 3,712 bytes (20 neurons) |
| Bytes/neuron | 185.6 |
| Arduino Uno | No (3,712 > 2,048 SRAM) |
| STM32F0 | **Yes** (4 KB SRAM) |
| ESP32 | **Yes** (520 KB SRAM) |

---

## Quick Start

```bash
git clone https://github.com/BEKO2210/cricket-brain.git
cd cricket-brain/use_cases/02_predictive_maintenance

# Synthetic demo (Normal + 3 fault types + mixed)
cargo run --release

# Classify CSV
cargo run --release -- --csv data/processed/sample_bearing.csv

# Benchmarks
cargo run --release --example bearing_sdt
cargo run --release --example bearing_latency
cargo run --release --example bearing_memory
cargo run --release --example bearing_stress

# 12 tests
cargo test
```

---

## Architecture

```
Vibration Signal ──→ Speed Compensation ──→ ResonatorBank (4 channels)
                     f × (cal/current)        │
                                        ┌─────┼─────┐─────┐
                                       FTF   BSF  BPFO  BPFI
                                       15Hz  69Hz 107Hz 162Hz
                                        └─────┼─────┘─────┘
                                              │
                                     Energy Accumulator (50-step window)
                                              │
                                         argmax(channels)
                                              │
                                 ┌────────┬───┴───┬──────────┐
                              Normal   Outer   Inner   Ball Defect
                                      Race    Race
```

### Bearing: SKF 6205-2RS

| Abbreviation | Frequency | Defect |
|---|---:|---|
| BPFO | 107.36 Hz | Ball Pass Frequency Outer race |
| BPFI | 162.19 Hz | Ball Pass Frequency Inner race |
| BSF | 69.04 Hz | Ball Spin Frequency |
| FTF | 14.83 Hz | Fundamental Train Frequency |

Calculated at 1797 RPM, 9 balls, 0.3126" ball diameter, 1.122" pitch diameter.

---

## API

### Rust

```rust
use cricket_brain_bearings::detector::{BearingDetector, FaultType};
use cricket_brain_bearings::vibration_signal;

let mut det = BearingDetector::new();

// Optional: set RPM for speed compensation
det.set_rpm(1200.0);

// Feed vibration frequency samples
for &freq in &vibration_signal::outer_race_fault(500) {
    if let Some(fault) = det.step(freq) {
        println!("{} (conf={:.2})", fault, det.confidence());
    }
}

// Batch: classify CSV file
let windows = vibration_signal::from_csv("data/processed/sample.csv");
let results = det.classify_stream(&windows, 25);
```

### Python (via cricket-brain PyO3)

```python
from cricket_brain import BrainConfig, Brain

config = BrainConfig()
config.min_freq = 15.0
config.max_freq = 162.0

brain = Brain(config)
for freq in vibration_frequencies:
    output = brain.step(freq)
    if output > 0.0:
        print(f"Fault detected: {output:.3f}")
```

---

## Dataset

| Field | Value |
|-------|-------|
| Name | CWRU Bearing Data Center |
| License | **Public Domain** |
| URL | https://engineering.case.edu/bearingdatacenter |
| Bearing | SKF 6205-2RS |
| Sampling | 12,000 Hz |
| Motor | 2 HP, 1797 RPM |
| Faults | Inner race, outer race, ball (0.007"–0.021") |

See [data/SOURCES.md](data/SOURCES.md) for download instructions.

---

## Project Structure

```
02_predictive_maintenance/
├── Cargo.toml
├── README.md                    # This file
├── CLAUDE.md                    # Build plan
├── data/
│   ├── raw/                     # CWRU .mat files (not committed)
│   ├── processed/
│   │   └── sample_bearing.csv   # 200 synthetic windows
│   └── SOURCES.md
├── src/
│   ├── lib.rs
│   ├── detector.rs              # BearingDetector + ConfusionMatrix
│   ├── vibration_signal.rs      # Signal generation + CSV I/O
│   └── main.rs                  # Demo binary (--csv mode)
├── benchmarks/
│   ├── bearing_sdt.rs           # d' and AUC
│   ├── bearing_latency.rs       # Throughput
│   ├── bearing_memory.rs        # RAM footprint
│   └── bearing_stress.rs        # Adversarial stress test
├── python/
│   ├── requirements.txt
│   ├── preprocess.py            # CWRU .mat → CSV via FFT
│   ├── evaluate.py              # F1 scores
│   └── plot_results.py          # 3 PNG plots
└── docs/
    ├── results.md               # Full benchmark results
    ├── limitations.md            # Honest failure analysis
    ├── fault_timeline.png
    ├── confusion_matrix.png
    └── confidence_dist.png
```

---

## Honest Limitations

1. **Single-fault only** — reports dominant channel, cannot detect simultaneous faults
2. **No severity estimation** — cannot distinguish defect sizes (0.007" vs 0.021")
3. **Speed compensation requires tachometer** — without RPM signal, only ±20% accuracy
4. **Synthetic data only** — not validated on real CWRU accelerometer signals
5. **No amplitude analysis** — frequency-only detection, no vibration level trending

See [docs/limitations.md](docs/limitations.md) for full analysis and
[docs/competitive_analysis.md](docs/competitive_analysis.md) for the
fully sourced breakdown against envelope analysis, TinyML CNNs
(FaultNet, Hakim 2023 Lite CNN), ResNet-50 and commercial SKF IMx.

---

## What this detector does NOT do

- **No severity estimation.** A 0.007" surface spall and a 0.021"
  through-wall crack at the same defect frequency report identically.
- **No simultaneous multi-fault reporting** in v0.1 (argmax only). v0.2
  channel-threshold work is tracked in the MASTER_PLAN backlog.
- **No amplitude / RMS trending.** Frequency-only detection.
- **No real CWRU-data validation yet.** All benchmarks are on synthetic
  fault-frequency signals; real `.mat` validation is the top-priority
  milestone for v0.2.
- **Not a replacement for envelope analysis, SKF IMx, or a full
  condition-monitoring suite.** Think of this as a fault-frequency
  triage front-end for a cheap wireless tag.

---

## How It Compares (2026-04-24)

> **Disclaimer — these systems do not all perform the same task.**
> CricketBrain classifies 4 defect-frequency regimes. Envelope analysis
> adds harmonic sidebands and severity trending. Lite CNN (Hakim 2023)
> and ResNet-50 classify 10 CWRU classes (4 fault types × 3 severities
> + normal). SKF IMx tracks ISO 10816 vibration levels + RUL. The
> comparison is one of **operating envelope** (RAM, power,
> explainability, training-data requirement), not of shared accuracy.

Short version — full breakdown with citations in
[docs/competitive_analysis.md](docs/competitive_analysis.md):

| System | RAM | Latency | Avg power @ 1 Hz | Training data | CWRU accuracy |
|--------|----:|--------:|-----------------:|---------------|--------------:|
| **CricketBrain UC02** | **3.7 KB** | **0.13-0.26 µs/step** | **< 1 µW compute** | **Zero** | 93.0% (synthetic) |
| Classical envelope analysis | < 5 KB | 1-10 ms | ~500 µW | Zero | 95-98% |
| Lite CNN (Hakim 2023) | ~100 KB | 120-140 ms | ~14 mW | CWRU full | 99.95% |
| ResNet-50 | > 1 GB GPU | ~3 s | offline | ImageNet + CWRU | 99.95% |
| SKF IMx / Emerson AMS | PC-class | — | mains (~10 W) | vendor corpora | — |

CricketBrain's niche: **wireless, self-powered bolt-on vibration tag**
on $2 STM32F0 where even the lightest TinyML CNN (~100 KB RAM, ~14 mW)
doesn't fit. CNNs win on accuracy (99.95% vs 93%) and severity
estimation, but need 250 × more memory and 20,000 × more compute.

---

## License

- **Source Code:** AGPL-3.0-only ([LICENSE](../../LICENSE))
- **Commercial:** Requires paid license ([COMMERCIAL.md](../../COMMERCIAL.md))
- **Dataset (CWRU):** Public Domain — no restrictions

---

## References

- [CricketBrain Whitepaper](../../RESEARCH_WHITEPAPER.md)
- [USE_CASES.md — Predictive Maintenance](../../USE_CASES.md#2-predictive-maintenance-for-industrial-bearings)
- [Website Demo](../../website/pages/bearings.html)
- [Benchmark Results](docs/results.md)
- [Limitations](docs/limitations.md)
- Loparo, K.A., "Bearings Data Center," Case Western Reserve University
