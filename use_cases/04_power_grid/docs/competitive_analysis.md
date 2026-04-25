# UC04 Power Grid — Competitive Analysis

**Date:** 2026-04-24 | **CricketBrain v3.0.0 (UC04)**

> **Disclaimer — not the same task.** CricketBrain triages 5
> categorical events on a single dominant-frequency stream. PMU-class
> instruments measure exact frequency to ±0.005 Hz, full RMS, phase,
> sequence components and individual harmonic amplitudes. Commercial
> power-quality monitors (Schneider PowerLogic, Fluke 1770) add
> sag/swell/transient classification. Accuracy numbers are **not
> directly comparable** — read the tables as an *operating-envelope*
> comparison (RAM, power, latency, training-data requirement,
> deployment cost).

How does CricketBrain compare to **classical FFT-based PQ analysers**,
to **commercial power-quality monitors**, to **PMUs**, and to
**TinyML** approaches for grid monitoring? All numbers below come from
vendor datasheets, IEC/IEEE standards, or peer-reviewed papers.

---

## TL;DR

| Budget | Who wins | Why |
|--------|----------|-----|
| **< $5 per node, < 1 mW, fleet of thousands** | **CricketBrain** | 3.7 KB RAM, 0.13 µs/step, zero training data |
| **$50-500 substation analyser** | Classical FFT PQ analyser | Full spectrum (50 lines), < 0.1 % THD precision |
| **PMU class** (synchronous-island detection, ±0.005 Hz) | **Schweitzer / GE / ABB PMU** | IEEE C37.118 compliant |
| **Commercial PQM** (sag/swell/voltage) | **Schneider PowerLogic, Fluke 1770** | Full IEC 61000-4-30 Class A |

CricketBrain's niche is the **first-screen, ubiquitous, edge-cheap
deployment** — a $5 sensor on every distribution-transformer secondary
that flags abnormalities for a $5,000 PQM to analyse.

---

## 1. RAM & Cost per Node

| System | RAM | Hardware | Approx cost / node |
|--------|----:|----------|------------------:|
| **CricketBrain UC04** | **3.7 KB** | STM32F0 + voltage divider | **< $5** |
| Classical FFT analyser (e.g. STM32F4 + DSP libs) | 64-256 KB | Cortex-M4 board | $50-100 |
| Commercial PQM (Schneider PowerLogic ION8650) | MB-class | Embedded Linux | ~$2,000 |
| Fluke 1770 series | MB-class | Industrial PC | ~$5,000 |
| PMU (Schweitzer SEL-487E, GE N60) | 1-4 MB | Custom hardened | ~$10,000+ |

CricketBrain is **10-1,000 ×** cheaper per deployed node and is the
only option that fits a 4 KB SRAM target.

---

## 2. Inference Latency

| System | Per-decision time | Notes |
|--------|------------------:|-------|
| **CricketBrain UC04** | **0.13-0.34 µs/step** (49 ms first-decision wall-clock) | Buffer-bound, not compute-bound |
| Classical FFT (1024-point on Cortex-M4) | 1-5 ms | per FFT window |
| Commercial PQM | 50-200 ms | Class A measurement chain |
| PMU | 20 ms (50 frame/s) | IEEE C37.118 Class P / M reporting rate |

---

## 3. Average Power at Continuous Monitoring

| System | Active power | Continuous duty | Avg power |
|--------|-------------:|----------------:|----------:|
| **CricketBrain — STM32F0 @ 48 MHz** | ~15 mW | 0.001 % (event-driven) | **< 0.5 µW compute** |
| Classical FFT on Cortex-M4 @ 80 MHz | ~50 mW | 1 % | ~500 µW |
| Commercial PQM (Linux Box) | ~5-10 W | 100 % | **5-10 W** |
| PMU | ~10-20 W | 100 % | **10-20 W** |

For a **harvester-powered** distribution-transformer monitor (typical
budget ~100 µW from CT-secondary current), only CricketBrain fits.

---

## 4. Training-Data Requirement

| System | Training data |
|--------|---------------|
| **CricketBrain UC04** | **Zero** (channels at integer multiples of 50 Hz, derived from physics) |
| Classical FFT + rule-based classifier | Zero |
| Commercial PQM | Zero (rule-based) |
| TinyML PQ classifier (CNN on PMU spectrograms) | ~100-1,000 labelled events / class |
| Full DNN PQ classifier | 10,000+ labelled events |

Several recent papers (Mahela et al. 2020, Kapoor et al. 2022) report
~95-99 % accuracy on 5-10-class PQ-event classification with CNN/LSTM
models, but they need labelled corpora and Cortex-M7+ hardware.

---

## 5. Standards & Compliance

| Standard | What it requires | CricketBrain status |
|----------|------------------|---------------------|
| IEEE 519 | Voltage/current distortion limits | Detects 2nd-4th harmonic dominance categorically; does **not** quantify individual amplitudes |
| IEC 61000-4-30 Class A | ±0.05 % frequency, 200 ms windows | **Not** Class A — categorical only |
| IEC 61000-4-7 | Harmonic measurement up to 50th | **No** — 4 channels only |
| IEEE C37.118 (PMU) | ±0.005 Hz, ±1 % TVE | **No** — triage class, not measurement class |

CricketBrain is **not a substitute** for any IEC/IEEE-compliant
instrument. It is a triage front-end: cheap, ubiquitous, and event-
flagging — leaving precision measurement to the dedicated tools it
alarms.

---

## 6. Operating-Envelope Summary

| Capability | CricketBrain | Classical FFT | TinyML PQ | Commercial PQM | PMU |
|------------|:-:|:-:|:-:|:-:|:-:|
| Categorical event triage (5 classes) | **Yes** | Yes | Yes | Yes | Partial |
| Exact frequency (±0.005 Hz) | No | Partial | No | Yes | **Yes** |
| Voltage / sag-swell / transient | No | With voltage chain | Yes | **Yes** | Yes |
| Phase / sequence / unbalance | No | Yes | Partial | **Yes** | Yes |
| 50+ harmonic lines | No | **Yes** | No | Yes | Yes |
| Runs on $2 STM32F0 | **Yes** | No | No | No | No |
| Runs on harvester power (< 1 mW) | **Yes** | No | No | No | No |
| Sub-µs compute per step | **Yes** | No | No | No | No |
| Deployable per distribution transformer | **Yes** | Partial | Partial | No | No |
| Zero training data | **Yes** | Yes | No | Yes | Yes |
| Explainable / deterministic | **Yes** | Yes | Partial | Yes | Yes |
| IEC 61000-4-30 Class A | No | Class S possible | No | **Yes** | Yes |

---

## 7. When to Pick Which

Use **CricketBrain** when:

- Deploying thousands of cheap edge sensors on the distribution network
  for first-screen anomaly detection.
- Hardware budget is < $5 / node, power budget < 1 mW.
- 5-class categorical triage (Outage / Nominal / H2 / H3 / H4) suffices.
- No training data is available.
- Output feeds a centralised analyser that does the precision
  measurement on alarm.

Use **Classical FFT PQ analyser** when:

- Cortex-M4+ hardware available.
- Need per-harmonic amplitudes up to the 25th-50th line.
- Substation-class deployment, not per-transformer.

Use **Commercial PQM (Schneider, Fluke)** when:

- IEC 61000-4-30 Class A compliance is required.
- Full sag/swell/transient/RMS chain is needed.
- Centralised dashboard + alarm management.

Use **PMU (Schweitzer, GE, ABB)** when:

- ±0.005 Hz frequency precision is required (synchronous-island
  detection, under-frequency load shedding).
- IEEE C37.118 Class P or M compliance.
- Wide-area measurement system (WAMS) integration.

---

## Sources

- [IEEE 519-2014 Recommended Practices for Harmonic Control](https://standards.ieee.org/standard/519-2014.html)
- [IEC 61000-4-30 Power-quality measurement methods](https://webstore.iec.ch/publication/68642)
- [EPFL DESL Smart Grid Lab](https://www.epfl.ch/labs/desl-pwrs/smart-grid/)
- [Mahela et al. — Power Quality Disturbances Classification, IEEE Access 2020](https://ieeexplore.ieee.org/document/9216612)
- [Kapoor et al. — Deep-learning for power-quality events, Energy Reports 2022](https://www.sciencedirect.com/science/article/pii/S2352484722002323)
- [Schneider PowerLogic ION9000](https://www.se.com/ww/en/product-range/61915-powerlogic-ion9000-series/)
- [Fluke 1770 series Power Quality Analysers](https://www.fluke.com/en-us/product/electrical-testing/power-quality/1770-series)
- [Schweitzer Engineering Laboratories SEL-487E](https://selinc.com/products/487E/)
