# CricketBrain Application: Marine Acoustic Monitoring

> **Status:** v0.1.0 | **CricketBrain v3.0.0** | **License:** AGPL-3.0 | **Date:** 2026-04-24

---

## Overview

The ocean is an increasingly noisy place. Baleen whales vocalise in the
same 10-500 Hz band that cargo-ship propellers dominate, and the IMO
expects a 30 % rise in shipping traffic by 2035. CricketBrain's
ResonatorBank classifies hydrophone signals into five categories — fin
whale, blue whale, ship noise, humpback song, or ambient — in 3,800 bytes
of RAM with zero training data.

**Market Size:** $4 B marine PAM | **Target:** ESP32 / STM32F0 ($2 MCU) on a solar-powered smart buoy

---

## Benchmark Results (2026-04-24)

### Classification Performance (`sample_marine.csv`, 200 windows)

| Class | TP | FP | Precision | Recall | F1 |
|---|---:|---:|---:|---:|---:|
| Ambient | 19 | 1 | 0.950 | 0.950 | 0.950 |
| Fin Whale (20 Hz) | 19 | 3 | 0.864 | 0.950 | 0.905 |
| Blue Whale (80 Hz) | 17 | 3 | 0.850 | 0.850 | 0.850 |
| Ship Noise (140 Hz) | 17 | 3 | 0.850 | 0.850 | 0.850 |
| Humpback Song (200 Hz) | 18 | 0 | 1.000 | 0.900 | 0.947 |
| **Macro Average** | | | **0.903** | **0.900** | **0.900** |

**Accuracy:** 90/100 = **90.0 %** | **d' (SDT):** 6.18 (EXCELLENT, all conditions)

### Ship Traffic Detection

| Transit Length | Ship Windows | Ship Ratio | Verdict |
|---:|---:|---:|---|
| 500 steps | 7 / 10 | 70.0 % | CPA correctly flagged |
| 1500 steps | 20 / 30 | 66.7 % | CPA correctly flagged |
| 3000 steps | 39 / 60 | 65.0 % | CPA correctly flagged |
| 5000 steps | 59 / 100 | 59.0 % | Long approach tails → Ambient |

In every transit, the closest-point-of-approach window is reliably flagged
as `ShipNoise`. The approach and recede tails produce `Ambient`, which is
the physically correct answer — the ship really is below the noise floor
at those ranges.

### Whale Under Ship Noise

A fin-whale vocalising through a simultaneous ship passage (real-ocean
scenario). Over 2000 steps the detector produces:

| Event | Windows |
|-------|--------:|
| FinWhale | 32 |
| ShipNoise | 8 |

The endangered species surfaces despite acoustic masking.

### Noise Resilience

| Noise % | Accuracy |
|--------:|---------:|
| 0-10 % | **100 %** |
| 20 % | 90 % DEGRADED |
| 30-50 % | 76-82 % DEGRADED |

### Sea-State Compensation

`set_sea_state(state)` raises the ambient threshold proportionally. Tested
on 1000-step ambient-noise runs: **100 % Ambient preservation at sea states 0
through 8** (zero false alarms in every tier).

### Performance

| Metric | Value |
|--------|-------|
| Latency | 0.13-0.28 µs/step |
| RAM | 3,712 bytes (20 neurons) |
| Struct | 88 bytes (stack) |
| Total | 3,800 bytes |
| Arduino Uno | No (3,712 > 2,048 SRAM) |
| STM32F0 | **Yes** (4 KB SRAM) |
| ESP32 | **Yes** (520 KB SRAM) |
| Smart Buoy 16 KB | **Yes (4× margin)** |

---

## Quick Start

```bash
git clone https://github.com/BEKO2210/cricket-brain.git
cd cricket-brain/use_cases/03_marine_acoustic

# Synthetic demo (5 scenarios + ambient → ship → whale sequence)
cargo run --release

# Dedicated ship-transit demo (cargo vessel sailing past the hydrophone)
cargo run --release -- --ship-transit

# Classify CSV
cargo run --release -- --csv data/processed/sample_marine.csv

# Benchmarks
cargo run --release --example marine_sdt
cargo run --release --example marine_latency
cargo run --release --example marine_memory
cargo run --release --example marine_stress

# 20 tests
cargo test
```

---

## Architecture

```
Hydrophone Audio ──→ STFT (512-sample window) ──→ Dominant Frequency ──→ ResonatorBank (4 channels)
                     ~128 ms / window                                     │
                                                                ┌─────┼─────┐─────┐
                                                               FIN  BLUE  SHIP  HUMP
                                                               20Hz 80Hz 140Hz 200Hz
                                                                └─────┼─────┘─────┘
                                                                      │
                                                             Energy Accumulator (50-step window)
                                                                      │
                                                       total < threshold?
                                                             │        │
                                                           yes        no
                                                             │        │
                                                          Ambient   argmax(channels)
                                                                      │
                                                  ┌─────────┬─────────┼─────────┬─────────┐
                                              FinWhale  BlueWhale  ShipNoise  Humpback
```

### Target Sources: MARS 10-500 Hz Band

| Channel | Frequency | Species / Source |
|---|---:|---|
| FIN | 20.0 Hz | Fin whale (*Balaenoptera physalus*) — 20-Hz stereotyped pulse |
| BLUE | 80.0 Hz | Blue whale (*Balaenoptera musculus*) — NE-Pacific A-call |
| SHIP | 140.0 Hz | Cargo-ship propeller cavitation peak (10-14 kn) |
| HUMP | 200.0 Hz | Humpback (*Megaptera novaeangliae*) — song unit |

These four frequencies sit inside MBARI's routine baleen-whale detection band.

---

## API

### Rust

```rust
use cricket_brain_marine::detector::{AcousticEvent, MarineDetector};
use cricket_brain_marine::acoustic_signal;

let mut det = MarineDetector::new();

// Optional: adapt to rough seas
det.set_sea_state(6);

// Feed hydrophone dominant frequencies
for &freq in &acoustic_signal::ship_passage(500) {
    if let Some(event) = det.step(freq) {
        println!("{} (conf={:.2})", event, det.confidence());
    }
}

// Batch: classify a preprocessed CSV
let windows = acoustic_signal::from_csv("data/processed/sample_marine.csv");
let results = det.classify_stream(&windows, 25);
```

### Python (via cricket-brain PyO3)

```python
from cricket_brain import BrainConfig, Brain

config = BrainConfig()
config.min_freq = 20.0
config.max_freq = 200.0

brain = Brain(config)
for freq in hydrophone_peak_frequencies:
    output = brain.step(freq)
    if output > 0.0:
        print(f"Marine event detected: {output:.3f}")
```

---

## Dataset

| Field | Value |
|-------|-------|
| Name | MBARI MARS Cabled Observatory Hydrophone |
| License | **CC BY 4.0** |
| URL | https://www.mbari.org/technology/mars/ |
| Location | Monterey Bay, CA — 891 m depth |
| Sampling | 256 kHz broadband (decimated to 2 kHz for baleen whale work) |
| Coverage | Continuous since 2015 (~2 PB available) |
| Hosting | AWS Open Data Sponsorship (`s3://pacific-sound-2khz/`) |

See [data/SOURCES.md](data/SOURCES.md) for download instructions.

---

## Project Structure

```
03_marine_acoustic/
├── Cargo.toml
├── README.md                       # This file
├── CLAUDE.md                       # Build plan
├── ci.yml                          # CI workflow
├── data/
│   ├── raw/                        # MARS .wav/.flac (not committed)
│   ├── processed/
│   │   └── sample_marine.csv       # 200 synthetic windows
│   └── SOURCES.md
├── src/
│   ├── lib.rs
│   ├── detector.rs                 # MarineDetector + ConfusionMatrix
│   ├── acoustic_signal.rs          # Signal generators + CSV I/O
│   └── main.rs                     # Demo binary (--csv / --ship-transit)
├── benchmarks/
│   ├── marine_sdt.rs               # d' and SDT analysis
│   ├── marine_latency.rs           # Throughput
│   ├── marine_memory.rs            # RAM footprint
│   └── marine_stress.rs            # Adversarial stress test
├── python/
│   ├── requirements.txt
│   ├── preprocess.py               # MARS .wav → CSV via STFT
│   ├── evaluate.py                 # F1 scores
│   └── plot_results.py             # 3 PNG plots
└── docs/
    ├── results.md                  # Full benchmark results
    ├── limitations.md              # Honest failure analysis
    ├── api.md                      # API reference
    ├── event_timeline.png
    ├── confusion_matrix.png
    └── confidence_dist.png
```

---

## Honest Limitations

1. **Single-label only** — reports the dominant channel; cannot flag
   simultaneous species.
2. **Boundary frequencies → Ambient** — sources between two tuned
   channels fall outside every Gaussian tuning curve.
3. **No source localisation** — answers *what*, not *where* or *how far*.
4. **Noise robustness caps at ~20 %** — sustained broadband contamination
   degrades accuracy.
5. **Synthetic data only in v0.1.0** — real MARS data validation planned
   for v0.2.

See [docs/limitations.md](docs/limitations.md) for full analysis.

---

## License

- **Source Code:** AGPL-3.0-only ([LICENSE](../../LICENSE))
- **Commercial:** Requires paid license ([COMMERCIAL.md](../../COMMERCIAL.md))
- **Dataset (MBARI MARS):** CC BY 4.0 — attribution required

---

## References

- [CricketBrain Whitepaper](../../RESEARCH_WHITEPAPER.md)
- [USE_CASES.md — Marine Acoustic](../../USE_CASES.md)
- [Website Demo](../../website/pages/marine.html)
- [Benchmark Results](docs/results.md)
- [Limitations](docs/limitations.md)
- Ryan, J.P. et al., "Oceanic giants dance to atmospheric rhythms,"
  *Geophysical Research Letters*, 2019.
- MBARI, "Monterey Accelerated Research System (MARS) underwater
  observatory," https://www.mbari.org/technology/mars/
