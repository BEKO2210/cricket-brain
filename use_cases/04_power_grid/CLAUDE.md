# UC04 Power Grid — CLAUDE Build Plan

> Local plan file for Use Case 04. Updated after every run.

---

## 1. Overview

A **power-quality triage core** for distribution-grid monitoring. The
EPFL Smart-Grid Lab publishes synchrophasor + 50 kHz aux waveform PMU
recordings of real distribution-network events; CricketBrain triages
each PMU window into one of five `GridEvent` categories using a
4-channel resonator bank tuned to the 50 Hz fundamental and its
2nd-4th harmonics.

No training data, 3,800 bytes of RAM, runs on a $2 STM32F0 — designed
as a wide-deployment edge sensor that flags abnormalities for a
centralised PQM to analyse.

---

## 2. Dataset: EPFL Smart-Grid Distribution Test Network

| Field | Value |
|-------|-------|
| Source | EPFL DESL Lab (École Polytechnique Fédérale de Lausanne) |
| URL | https://www.epfl.ch/labs/desl-pwrs/smart-grid/ |
| License | CC BY 4.0 |
| Equipment | OpenPMU + EPFL synchrophasor units |
| Sampling | 50 frame/s synchrophasor + 50 kHz aux waveform |
| Format | CSV / HDF5 |

### Channel mapping

- FUND — 50 Hz (healthy fundamental)
- H2 — 100 Hz (DC offset, transformer in-rush)
- H3 — 150 Hz (non-linear loads — most common PQ issue)
- H4 — 200 Hz (switching artefacts, fast EMI)

These four frequencies are exact integer multiples of 50 Hz so the
`TokenVocabulary::new(&[...], 50.0, 200.0)` distribution lands precisely
on each tuned channel.

---

## 3. CricketBrain Approach

```rust
let vocab = TokenVocabulary::new(
    &["FUND", "H2", "H3", "H4"],
    50.0,
    200.0,
);
let mut bank = ResonatorBank::new(&vocab);
let outputs = bank.step(input_freq);
// outputs[0] > 0 → fundamental
// outputs[1] > 0 → 2nd harmonic
// outputs[2] > 0 → 3rd harmonic
// outputs[3] > 0 → 4th harmonic
```

`GridDetector` accumulates channel energy over a 50-step window,
returns `Outage` when total < `outage_threshold` (default 0.10),
otherwise emits the `argmax` channel as a `GridEvent`. v0.2-style
`with_bandwidth(0.20)` and `step_multi()` are available from day one.

---

## 4. Ten-Run Plan

| Run | Deliverable | Status |
|-----|-------------|--------|
| 1 | Scaffold (Cargo.toml, src/, SOURCES, sample CSV) | DONE 2026-04-24 |
| 2 | Data pipeline (Python STFT, CSV I/O) | DONE 2026-04-24 |
| 3 | Core detector (CSV classify + ConfusionMatrix) | DONE 2026-04-24 |
| 4 | Benchmark suite (SDT, latency, memory, stress) | DONE 2026-04-24 |
| 5 | Python analysis (evaluate.py + 3 plots) | DONE 2026-04-24 |
| 6 | Stress test (factory startup, brownout, noise) | DONE 2026-04-24 |
| 7 | Website demo (`grid.html`) | DONE 2026-04-24 |
| 8 | Full README + docs/api.md + competitive_analysis.md | DONE 2026-04-24 |
| 9 | CI workflow (`ci.yml`) | DONE 2026-04-24 |
| 10 | Metrics finalization (shared/metrics.json) | DONE 2026-04-24 |

---

## 5. Measured Results (2026-04-24)

| Metric | Value |
|--------|-------|
| Synthetic-window accuracy | 90.0 % |
| Macro F1 | 0.900 |
| d' (SDT, log-linear) | 6.18 (all 5 conditions EXCELLENT) |
| Latency | 0.13-0.34 µs/step |
| First decision | 49 ms |
| RAM | 3,712 bytes (20 neurons) |
| Detector struct | 88 bytes |
| Noise tolerance | 100 % up to 30 %, 98 % at 50 % |
| Tests passing | 18 / 18 |

---

## 6. Run Status

| Run | Status | Date | Notes |
|-----|--------|------|-------|
| 1 | DONE | 2026-04-24 | Cargo.toml, src/, 18/18 tests pass |
| 2 | DONE | 2026-04-24 | STFT pipeline, CSV I/O, sample_grid.csv |
| 3 | DONE | 2026-04-24 | 90 % CSV accuracy, 5-class ConfusionMatrix |
| 4 | DONE | 2026-04-24 | d'=6.18 all EXCELLENT, 0.13-0.34 µs/step, 3,712 B |
| 5 | DONE | 2026-04-24 | evaluate.py, plot_results.py (3 plots) |
| 6 | DONE | 2026-04-24 | Factory startup, brownout, noise + multi-harmonic |
| 7 | DONE | 2026-04-24 | website/pages/grid.html |
| 8 | DONE | 2026-04-24 | Full README + docs/api.md + competitive_analysis.md |
| 9 | DONE | 2026-04-24 | ci.yml — build/test/benchmarks/python-syntax |
| 10 | DONE | 2026-04-24 | shared/metrics.json updated with measured values |

---

## 7. License

- Source code: AGPL-3.0-only
- EPFL data: CC BY 4.0 (attribution required)
