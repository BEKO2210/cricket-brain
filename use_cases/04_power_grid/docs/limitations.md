# UC04 Power Grid — Known Limitations

**Date:** 2026-04-24 | **CricketBrain v3.0.0**

---

## 1. Categorical, Not Precise

The detector reports **categories** (Nominal vs each harmonic vs
Outage), **not** the exact fundamental frequency.

From the off-nominal sweep in `grid_stress::E`:

| Input | Detected |
|------:|----------|
| 49.5 Hz | Nominal |
| 49.8 Hz | Nominal |
| 50.0 Hz | Nominal |
| 50.2 Hz | Nominal |
| 50.5 Hz | Nominal |
| 51.0 Hz | Nominal |

A real synchronous-island detection (which needs to flag a 49.7 Hz
under-frequency event for under-frequency load shedding, UFLS) requires
a dedicated PMU with ±0.005 Hz precision. **CricketBrain triages, the
PMU measures.**

**Mitigation:** combine the two — feed CricketBrain's categorical
output to alarm logic, and take the precise frequency value from a
co-deployed PMU when an alarm fires.

---

## 2. No Voltage / Sag-Swell / Interruption Distinction

The detector consumes **frequency** only. It cannot distinguish:

- **Sag** (10-90 % voltage for 0.5 cycles to 1 min)
- **Swell** (110-180 % voltage)
- **Interruption** (<10 % voltage)
- **Transient** (impulsive over-voltage spike)
- **Notch** (commutation notch from rectifier loads)

All of these can occur with a clean 50 Hz fundamental and would be
classified as `Nominal` by this detector.

**Mitigation:** combine with a voltage-amplitude monitor; raise an
alarm on (Nominal frequency + abnormal voltage RMS).

---

## 3. No Phase / Sequence / Unbalance Analysis

Three-phase analysis (positive / negative / zero sequence components)
is not in scope. The detector treats each phase independently as a
single frequency stream.

**Real grid problems missed:** voltage unbalance > 2 % (IEEE 1159
Class A limit), phase loss, single-phasing of three-phase motors.

---

## 4. Single-Label Output (in v0.1; v0.2 mitigation available)

`GridDetector::step` reports only the dominant channel. Mixed-grid
scenes (50 Hz fundamental + 3rd-harmonic VFD load — a very common
configuration) get classified as whichever channel currently dominates.

**v0.2 mitigation (already implemented):** `step_multi` returns a
[`MultiLabelDecision`] flagging every channel above
`channel_threshold` independently. Verified by the
`multi_label_recovers_both_in_mixed_grid` test on a 70/30 mix.

---

## 5. Noise Robustness (GOOD on synthetic, real-world TBD)

| Synthetic noise % | Accuracy |
|-----:|---------:|
| 0–30 % | 100 % |
| 50 % | 98 % |

This characterises behaviour under **synthetic random-spike noise**.
Real PMU data additionally exhibits sustained broadband interference,
DC-offset drift, harmonic chorusing from neighbouring loads, and
inter-harmonics from arc furnaces. Real-data validation is pending.

---

## 6. Synthetic Data Only

All v0.1 benchmarks are on synthetic 50 Hz signals with:
- Pure tonal harmonics (no inter-harmonics)
- ±2 % frequency jitter (matches IEC 61000-4-30 Class A budget)
- No load step-changes between windows
- No voltage / phase information

Real EPFL PMU validation is the v0.2 priority milestone.

---

## 7. CricketBrain vs. Established Power-Quality Tools

| Capability | CricketBrain | Classical FFT-based PQ analyser | Commercial PQM (Schneider PowerLogic, Fluke 1770) |
|------------|:---:|:---:|:---:|
| 50 Hz fundamental triage | Yes | Yes | Yes |
| 2nd-5th harmonic detection | Yes (4 ch) | Yes (50+ ch) | Yes (full spectrum) |
| Exact frequency (±0.005 Hz) | No | Partial | **Yes** (PMU class) |
| Voltage / sag-swell | No | Yes | **Yes** |
| Phase / sequence / unbalance | No | Yes | **Yes** |
| Inter-harmonic (e.g. 175 Hz) | No | Partial | **Yes** |
| Runs on $2 STM32F0 (4 KB SRAM) | **Yes** | No | No |
| Continuous edge monitoring (< 1 mW) | **Yes** | Partial | No (mains) |
| Training data required | **Zero** | No | No |
| Sub-µs compute per step | **Yes** | No | No |
| Cost per node | **< $5** | $50-500 | $500-5000 |

CricketBrain's niche: **wide-deployment edge triage** — a cheap
distributed sensor on every distribution-transformer secondary, raising
flags that a centralised PQM analyses in detail. For a fully sourced
comparison see [docs/competitive_analysis.md](competitive_analysis.md).
