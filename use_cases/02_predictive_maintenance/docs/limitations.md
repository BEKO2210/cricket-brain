# UC02 Predictive Maintenance — Known Limitations

**Date:** 2026-04-10 | **CricketBrain v3.0.0**

---

## 1. Speed-Dependent Calibration (CRITICAL)

| RPM | BPFO (Hz) | Detected | Correct? |
|----:|----------:|----------|:--------:|
| 900 | 53.6 | Normal | No |
| 1200 | 71.5 | Ball Defect | No |
| 1500 | 89.3 | Normal | No |
| **1797** | **107.0** | **Outer Race** | **Yes** |
| 2100 | 125.0 | Outer Race | Yes |
| 2400 | 142.9 | Inner Race | No |

**Root cause:** Bearing defect frequencies are proportional to shaft speed. The ResonatorBank is tuned to fixed frequencies (calculated at 1797 RPM). At different speeds, the fault frequencies shift out of the Gaussian tuning window.

**Impact:** The detector only works within ±20% of the calibration speed. Real machines vary speed continuously (variable frequency drives, load changes).

**Mitigation (IMPLEMENTED):** `BearingDetector::set_rpm(rpm)` applies speed compensation by scaling input frequencies: `f_compensated = f_input × (cal_rpm / current_rpm)`. This maps fault frequencies at any shaft speed back to the calibration frequencies.

**Result with compensation:**

| RPM | Without Compensation | With Compensation |
|----:|:--------------------:|:-----------------:|
| 900 | Normal (WRONG) | **Outer Race (CORRECT)** |
| 1200 | Ball Defect (WRONG) | **Outer Race (CORRECT)** |
| 1500 | Normal (WRONG) | **Outer Race (CORRECT)** |
| 1797 | Outer Race | Outer Race |
| 2100 | Outer Race | Outer Race |
| 2400 | Inner Race (WRONG) | **Outer Race (CORRECT)** |

**6/6 correct with compensation** (vs 2/6 without). Requires a tachometer signal or speed estimation.

---

## 2. Single-Fault Detection Only

The detector reports the **dominant** fault channel. When two faults coexist:

| Mixed Faults | Detected | Missed |
|---|---|---|
| Outer + Inner | Inner Race | Outer Race |
| Outer + Ball | Ball Defect | Outer Race |
| Inner + Ball | Ball Defect | Inner Race |

**Root cause:** The classification picks the channel with maximum accumulated energy. Simultaneous faults compete rather than being reported in parallel.

**Impact:** Real bearings often develop multiple faults (e.g., outer race damage causes ball damage). Missing the secondary fault delays maintenance.

**Mitigation:** Report all channels exceeding a minimum energy threshold instead of only the maximum. This would require a multi-label classification approach.

---

## 3. No Severity Estimation

The detector outputs a binary fault/no-fault decision per channel. It does NOT estimate:
- Fault size (0.007" vs 0.021" defect diameter)
- Fault progression rate
- Remaining useful life (RUL)

**Impact:** Cannot prioritize maintenance scheduling. A severe fault and a minor fault produce the same output.

**Mitigation:** Track spike energy trends over time. Increasing energy in a fault channel correlates with fault severity in the CWRU dataset.

---

## 4. Noise Resilience (GOOD)

| Noise | Accuracy | Verdict |
|------:|---------:|---------|
| 0% | 100% | OK |
| 5% | 100% | OK |
| 10% | 100% | OK |
| 20% | 100% | OK |
| 30% | 100% | OK |
| 50% | 100% | OK |

**Surprisingly robust:** The 50-step energy accumulation window provides excellent noise averaging. Individual random spikes are diluted by the sustained fault frequency energy. This is a major advantage over instantaneous detectors.

---

## 5. Synthetic Data Only

All results are on synthetic vibration signals with:
- Perfect frequency purity (single dominant frequency per segment)
- No amplitude modulation (real faults show modulation at shaft speed)
- No bearing natural frequencies (resonances of the bearing structure)
- No motor electrical noise (50/60 Hz harmonics)

Real CWRU data would test: signal-to-noise ratio, spectral leakage, harmonic interference, load-dependent amplitude changes.

---

## 6. CricketBrain vs. Traditional Approaches

| Capability | CricketBrain | Envelope Analysis | Deep Learning |
|-----------|:---:|:---:|:---:|
| Fixed-speed fault detection | Yes | Yes | Yes |
| Variable-speed detection | No | With order tracking | Yes |
| Fault severity estimation | No | Yes (amplitude) | Yes |
| Multiple simultaneous faults | No | Limited | Yes |
| Training data required | No | No | Yes (large datasets) |
| Runs on MCU ($0.50) | Yes | Sometimes | No |
| Sub-microsecond latency | Yes | No | No |
| Explainable decisions | Yes | Yes | Partially |
