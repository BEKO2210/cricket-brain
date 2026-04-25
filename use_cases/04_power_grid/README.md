# CricketBrain Application: Power-Grid Harmonic-Pattern Triage

> **Status:** v0.1.0 research prototype | **CricketBrain v3.0.0** | **License:** AGPL-3.0 | **Date:** 2026-04-24
>
> **Validation status:** Synthetic 50 Hz frequency-pattern streams only.
> Real EPFL Smart-Grid PMU validation is planned for v0.2 — see
> [MASTER_PLAN.md](../MASTER_PLAN.md).
>
> **What this detects vs. what it does not.** CricketBrain takes a single
> dominant-frequency time-series as input and flags which harmonic-frequency
> band is currently dominant. It does **not** measure calibrated harmonic
> amplitudes, THD, phase, sag/swell or IEC 61000-4-30 Class-A power-quality
> quantities. Treat the output as **harmonic-like frequency-pattern triage,
> not calibrated harmonic metrology**.

---

## Overview

A 4-channel **harmonic-pattern triage core** for distribution-grid
event detection. CricketBrain's ResonatorBank flags each PMU window as
one of five `GridEvent` categories — Outage / Nominal / 2nd / 3rd / 4th
harmonic dominant — in 3,800 bytes of RAM with zero training data.

**What it is:** A wide-deployment first-screen front-end for cheap
embedded triage nodes that flags potential harmonic-distortion or
outage events for a downstream IEC 61000-4-30 Class-A analyser to
quantify.

**What it is NOT:** An IEC 61000-4-30 Class-A measurement instrument,
a PMU, a sag/swell/voltage analyser, a phase/sequence analyser, an
amplitude / THD / phase / interharmonic / supraharmonic measuring
device. See [What this detector does NOT do](#what-this-detector-does-not-do).

### Market context

Smart-grid monitoring is a ~$100 B annual segment (BloombergNEF, 2024
estimate). **Hardware target:** designed for **<$5-class embedded
triage nodes** mounted at distribution-transformer secondaries
(STM32F0+ / ESP32). *Hardware BOM, certification path and field-trial
deployment costs are pending and not part of this v0.1 release.*

---

## Benchmark Results (2026-04-24, synthetic 50 Hz frequency-pattern streams)

All numbers below are measured on **synthetic 50 Hz signals** — not on
real EPFL PMU recordings. Real-data validation is pending.

### Classification Performance (`sample_grid.csv`, 200 synthetic windows)

| Class | TP | FP | Precision | Recall | F1 |
|---|---:|---:|---:|---:|---:|
| Outage | 19 | 1 | 0.950 | 0.950 | 0.950 |
| Nominal (50 Hz) | 19 | 3 | 0.864 | 0.950 | 0.905 |
| 2nd Harmonic (100 Hz) | 17 | 3 | 0.850 | 0.850 | 0.850 |
| 3rd Harmonic (150 Hz) | 17 | 3 | 0.850 | 0.850 | 0.850 |
| 4th Harmonic (200 Hz) | 18 | 0 | 1.000 | 0.900 | 0.947 |
| **Macro Average** | | | **0.903** | **0.900** | **0.900** |

**Synthetic-window accuracy:** 90 / 100 = **90.0 %** | **d' (SDT) [†]:** 6.18

[†] d' uses log-linear correction for ceiling hit-rates and floor
false-alarm rates (`[0.5/n, 1 − 0.5/n]` clipping; Hautus 1995). Without
the correction, perfect TPR = 1.000 / FPR = 0.000 cells would yield an
undefined / infinite d'.

### Factory-Startup Detection

| Disturbance length | H3 windows | Total | H3 ratio |
|---:|---:|---:|---:|
| 200 steps | 4 / 30 | 13.3 % |
| 400 steps | 8 / 30 | 26.7 % |
| 600 steps | 12 / 30 | 40.0 % |
| 800 steps | 16 / 30 | 53.3 % |
| 1200 steps | 24 / 36 | 66.7 % |

The H3-window ratio scales linearly with disturbance length — clean
separation from the surrounding nominal sections.

### Rolling-Brownout Detection

Each scheduled dip produces exactly one Outage window:

| Dips | Dip length | Outage windows | Outage % |
|---:|---:|---:|---:|
| 2 | 60 | 2 | 5.0 % |
| 4 | 80 | 4 | 10.0 % |
| 6 | 100 | 6 | 15.0 % |
| 10 | 120 | 10 | 25.0 % |

### Noise Resilience (synthetic random-spike transients only)

| Synthetic noise % | Accuracy |
|-----:|---------:|
| 0–30 % | **100 %** |
| 50 % | 98 % |

The 50-step energy accumulation window absorbs random transients
extremely well. **Caveat:** these numbers are on synthetic random-spike
noise, not on real PMU recordings with sustained broadband interference.

### Performance

| Metric | Value |
|--------|-------|
| Latency | 0.13-0.34 µs/step |
| RAM | 3,712 bytes (20 neurons) |
| Struct | 88 bytes (stack) |
| Total | 3,800 bytes |
| Arduino Uno | No (3,712 > 2,048 SRAM) |
| STM32F0 | **Yes** (4 KB SRAM) |
| Substation gateway 64 KB | **Yes (15× margin)** |
| PMU 1 MB | **Yes (250× margin)** |

---

## Quick Start

```bash
git clone https://github.com/BEKO2210/cricket-brain.git
cd cricket-brain/use_cases/04_power_grid

# Synthetic demo (5 scenarios + nominal → outage → 3rd-harm → recovery)
cargo run --release

# Factory-startup transient (3rd-harmonic VFD load comes online)
cargo run --release -- --factory

# Rolling brownout (4 dips inside a nominal grid)
cargo run --release -- --brownout

# Classify CSV
cargo run --release -- --csv data/processed/sample_grid.csv

# Benchmarks
cargo run --release --example grid_sdt
cargo run --release --example grid_latency
cargo run --release --example grid_memory
cargo run --release --example grid_stress

# 18 tests
cargo test
```

---

## Architecture

```
PMU Stream ──→ STFT (1024-sample window @ 4 kHz) ──→ Dominant Frequency ──→ ResonatorBank (4 channels)
              ~256 ms / window                                              │
                                                                ┌─────┼─────┐─────┐
                                                              FUND  H2    H3    H4
                                                              50Hz 100Hz 150Hz 200Hz
                                                                └─────┼─────┘─────┘
                                                                      │
                                                            Energy Accumulator (50-step window)
                                                                      │
                                                       total < threshold?
                                                             │        │
                                                           yes        no
                                                             │        │
                                                          Outage   argmax(channels)
                                                                      │
                                                  ┌─────────┬─────────┼─────────┬─────────┐
                                              Nominal      H2          H3       H4
                                              50 Hz      100 Hz      150 Hz   200 Hz
```

### Channel meaning (50 Hz EU grid)

| Channel | Frequency | Real-world cause |
|---|---:|---|
| FUND | 50 Hz | Healthy grid |
| H2 | 100 Hz | DC offset, transformer in-rush, half-wave rectifiers |
| H3 | 150 Hz | **Most common PQ issue** — non-linear loads (VFDs, SMPS, LED ballasts, arc) |
| H4 | 200 Hz | Switching artefacts, fast EMI from RF power-electronics |

For 60 Hz systems (US, Canada, Japan partial) remap to 60/120/180/240 Hz
in `src/detector.rs`.

---

## API

### Rust

```rust
use cricket_brain_grid::detector::{GridDetector, GridEvent};
use cricket_brain_grid::grid_signal;

let mut det = GridDetector::with_bandwidth(0.20); // v0.2 wider tuning

for &freq in &grid_signal::factory_startup(1500, 500) {
    if let Some(event) = det.step(freq) {
        println!("{} (conf={:.2})", event, det.confidence());
    }
}

// v0.2 multi-label: report Nominal AND H3 simultaneously when both active
while let Some(d) = det.step_multi(freq) {
    // d.events may contain [Nominal, ThirdHarmonic] in mixed-grid windows
}
```

### Python (via cricket-brain PyO3)

```python
from cricket_brain import BrainConfig, Brain

config = BrainConfig()
config.min_freq = 50.0
config.max_freq = 200.0

brain = Brain(config)
for freq in pmu_dominant_frequencies:
    output = brain.step(freq)
    if output > 0.0:
        print(f"Grid event detected: {output:.3f}")
```

---

## Dataset

| Field | Value |
|-------|-------|
| Name | EPFL Smart-Grid Distribution Test Network |
| License | **CC BY 4.0** |
| URL | https://www.epfl.ch/labs/desl-pwrs/smart-grid/ |
| Equipment | OpenPMU + EPFL synchrophasor units |
| Sampling | 50 frame/s synchrophasor + 50 kHz aux waveform |

See [data/SOURCES.md](data/SOURCES.md) for download instructions.

---

## What this detector does NOT do

**Measurement class — explicitly not in scope:**

- **No calibrated harmonic amplitudes.** CricketBrain reports *which*
  harmonic band is dominant, not *how strong* it is in volts or
  amperes. A 1 % H3 distortion and a 25 % H3 distortion both produce
  the same `ThirdHarmonic` label.
- **No THD computation.** No total-harmonic-distortion percentage,
  no individual line amplitudes h0-h50, no IEC 61000-4-7
  10-min / 3-s grouping bins.
- **No interharmonics or supraharmonics.** Only the four canonical
  channels at 50 / 100 / 150 / 200 Hz.
- **No exact frequency measurement.** 49.5-50.5 Hz all → `Nominal`.
  For ±0.005 Hz precision use a dedicated PMU.
- **No voltage / sag-swell / transient classification.** Frequency-only
  input — no RMS, no envelope, no impulse detection.
- **No phase / sequence / unbalance analysis.** Single-stream input.
- **No IEC 61000-4-30 Class-A compliance.** Triage class, not
  measurement class. Not certified to any IEC/IEEE PQ standard.
- **No real EPFL PMU data validation yet** — top-priority v0.2 milestone.

**The mental model:** CricketBrain triages frequency-pattern signatures
of harmonic-like events; it does not measure harmonics. The output is
an **alarm flag** that an IEC-Class-A instrument can quantify on
dispatch.

See [docs/limitations.md](docs/limitations.md) for full analysis and
[docs/competitive_analysis.md](docs/competitive_analysis.md) for the
fully sourced comparison against PMUs, commercial PQMs, classical FFT
analysers and TinyML PQ classifiers.

---

## How It Compares (2026-04-24)

> **CricketBrain is not a replacement for Class-A PQ analysers or PMUs.**
> It targets the **earlier layer in the measurement chain**: cheap,
> always-on event triage *before* expensive measurement equipment is
> dispatched. Fluke 1770 / Schneider PowerLogic / Schweitzer SEL-487E
> remain the right tools for IEC-compliant amplitude, THD, sag/swell,
> phase and synchrophasor measurement; CricketBrain is the front-end
> that decides *when* those tools should look.
>
> The systems below do not perform the same task — read the table as an
> *operating-envelope* comparison (RAM, power, latency, target tier),
> not a shared-accuracy comparison.

Short version — full breakdown with citations in
[docs/competitive_analysis.md](docs/competitive_analysis.md):

| System | RAM | Latency | Avg power @ 1 Hz | Tier |
|--------|----:|--------:|-----------------:|------|
| **CricketBrain UC04** (event triage) | **3.7 KB** | **0.13-0.34 µs/step** | **< 0.5 µW compute** | <$5 embedded triage node (BOM TBD) |
| Classical FFT analyser (Cortex-M4 + DSP libs) | 64-256 KB | 1-5 ms | ~500 µW | Substation gateway, $50-100 board |
| Commercial PQM (Schneider PowerLogic, Fluke 1770) | MB-class Linux | 50-200 ms | 5-10 W mains | IEC-Class-A measurement, $2 k-5 k |
| PMU (Schweitzer / GE / ABB, IEEE C37.118) | 1-4 MB | 20 ms | 10-20 W mains | ±0.005 Hz, $10 k+ |

The cost / RAM / power ranges are order-of-magnitude tier indicators
from vendor datasheets, *not* a one-to-one head-to-head benchmark; per-
deployment economics depend on hardware BOM, certification, sensors,
network and labour costs that are out of scope for this v0.1 release.

---

## Honest Limitations

1. **Triage, not metrology** — flags which harmonic band is dominant,
   does **not** quantify amplitude, THD or phase.
2. **Categorical, not precise** — 49.5-50.5 Hz all → Nominal; no
   ±0.005 Hz precision.
3. **No voltage / sag-swell / transient / interharmonic / supraharmonic
   detection** — frequency input only.
4. **No phase / sequence / unbalance analysis** — single-stream input.
5. **Single-label by default** — v0.2 `step_multi` recovers
   simultaneous fundamental + harmonic.
6. **Synthetic data only in v0.1** — real EPFL PMU validation pending.
7. **Not certified to any IEC/IEEE standard.** Pre-screen front-end
   only.

Full analysis: [docs/limitations.md](docs/limitations.md).

---

## License

- **Source Code:** AGPL-3.0-only ([LICENSE](../../LICENSE))
- **Commercial:** Requires paid license ([COMMERCIAL.md](../../COMMERCIAL.md))
- **Dataset (EPFL):** CC BY 4.0 — attribution required

---

## References

- [CricketBrain Whitepaper](../../RESEARCH_WHITEPAPER.md)
- [USE_CASES.md — Power Grid](../../USE_CASES.md)
- [Website Demo](../../website/pages/grid.html)
- [Benchmark Results](docs/results.md)
- [Competitive Analysis](docs/competitive_analysis.md)
- [Limitations](docs/limitations.md)
- IEEE 519-2014 — Recommended Practices for Harmonic Control
- IEC 61000-4-30 — Power Quality Measurement Methods
