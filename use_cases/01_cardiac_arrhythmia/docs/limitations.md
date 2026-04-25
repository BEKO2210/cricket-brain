# UC01 Cardiac Arrhythmia — Known Limitations

**Date:** v0.2 hardening | **CricketBrain core v3.0.0**

> Scientific integrity requires transparent documentation of failure modes.

---

## 0. v0.5 AAMI EC57:2012 DS2 findings (2026-04-25)

Full inter-patient evaluation on the canonical AAMI DS2 split:
22 records, 49 584 annotation beats, 42 510 detector emissions.

**Pooled metrics**

| | Value |
|---|---:|
| Pooled accuracy | 96.60 % |
| Macro-F1 (4 classes, all with support) | 0.934 |
| Balanced accuracy | 0.936 |
| Recall: Normal | 0.978 |
| Recall: Tachy | 0.946 |
| Recall: Brady | 0.980 |
| Recall: Irregular | 0.841 |

**Honest constraints exposed by v0.5**

- **CricketBrain does not beat a simple rule.** The
  `ThresholdBurstBaseline` (band-gate + RR-window, ~50 lines)
  reaches **97.53 %** on the same DS2 set (~1 pp above
  CricketBrain). The neuromorphic core's value reduces to:
  match rule-based accuracy in 928 bytes deterministically with
  zero training. The 1-second-window rule fails badly (24.7 %),
  so the rate-regime task itself is non-trivial — but a careful
  classical rule is sufficient.
- **Per-record range is wide.** 86.5 % (record 233 — frequent
  PVCs + AF) to 100.0 % (records 100, 103, 111). PVC-rich (200,
  214, 233) and AF-rich (232) records cap the accuracy.
- **Irregular recall is the weakest class** at 0.841. Some AF
  segments with relatively stable short RR have CV(RR) below 0.30
  and get classified as Tachy or Normal — this is a definition
  question of the rate-regime ground truth, not strictly a
  detector error.
- **DS1 not evaluated.** CricketBrain has no training phase, so
  DS1/DS2 are used purely as a record list. Running on DS1 would
  give a different statistical sample but no "training" effect.
  v0.6+ may add DS1 for symmetry.
- **No Pan-Tompkins / Tiny CNN reference yet.** Comparisons are
  to two in-tree rule baselines only. External references on the
  same DS2 set are tracked as v0.5-followup.

## 0a. v0.4 real-data findings (2026-04-25, superseded)

First real MIT-BIH run on records 100, 105, 200, 217, 232 — 11 375
annotation beats. Pooled accuracy **96.08 %** (9 549 / 9 939
emissions), macro-F1 **0.793**, balanced accuracy **0.819**.
Per-class recall (where ground truth has support): Normal 0.971,
Tachy 0.570, Irregular 0.916.

Honest constraints uncovered:

- **Tachycardia recall is only 57 % in this set.** Most fast-RR
  windows in MIT-BIH are part of AF episodes, which the rate-regime
  ground truth (CV(RR) > 0.30) labels Irregular. The detector
  agrees with that labelling, so "Tachy → Irregular" is the
  dominant tachy mis-call, not a free-running speed mistake. This
  is a definition of the rate-regime ground truth, not a detector
  failure per se.
- **Bradycardia has zero ground-truth support in this subset.** No
  performance claim is possible. Adding records that are
  predominantly slow (213, parts of 232) is a v0.4-followup item.
- **Per-record variability matters.** Record 100 (mostly normal)
  scored 100 %; records 200 (PVCs) and 232 (AF) dropped to ~91 %.
  Single headline numbers hide this — the README and website show
  the per-record breakdown.
- **No inter-patient train/test split is asserted yet.** The
  benchmark accepts a directory and treats it pooled; the user is
  responsible for splitting train/test patients. AAMI EC57:2012
  inter-patient evaluation will be added in v0.5.
- **Sample size is small (5 records / 11 375 beats).** Cross-record
  variance is real — a single seed / record selection is not a
  population estimate. The roadmap calls for cross-seed robustness
  (v0.7) and 10+ records (v0.4-followup).

## 0b. v0.2 audit findings

The v0.2 benchmark hardening replaced the legacy circular confusion
matrix with truth-based metrics and added 7-dimension stress sweeps,
two rule baselines, and a reject-aware coverage curve. The biggest
findings are:

- **The legacy 92.5 % accuracy was partly circular.** Truth was
  derived from the detector's own BPM estimate. The new truth-based
  4-class accuracy on the equivalent synthetic scenario is **~87 %**
  (after a 2-emission per-segment warmup) with macro-F1 ≈ 0.88.
- **Morphology jitter at 5 % already breaks detection.** Per-cycle
  scaling of P/QRS/T frequencies and durations by ±5 % drives
  macro-F1 below 0.10. The coincidence gate is tightly tuned to the
  carrier; modest morphology drift defeats it. Fix is on the
  [BENCHMARK_ROADMAP.md](../BENCHMARK_ROADMAP.md) v0.6 ablation
  milestone.
- **CricketBrain ≈ rule baseline on clean data.** A simple
  band-gate + RR-window threshold rule
  (`baselines::ThresholdBurstBaseline`) matches CricketBrain exactly
  on clean synthetic streams, and *outperforms* it under 2 %
  in-band noise. The v0.2 baseline benchmark surfaces this fact
  rather than hiding it.
- **In-band noise > 5 % spike probability** drives accuracy below
  0.20; the noise model is per-sample probability of a random in-band
  replacement, *not* the legacy whole-stream replacement model — the
  two are not directly comparable.
- **Reject-aware operating point is solid.** At confidence ≥ 0.9 the
  detector rejects ~39 % of decisions but is 100 % correct on the
  remainder (single-seed, `cardiac_reject_curve.csv`).

The v0.1 numbers below are kept for traceability.

---

## 1. Noise Sensitivity (mitigated with Preprocessor)

### Without Preprocessor (raw CricketBrain)

| Noise Injection | Accuracy | Verdict |
|----------------:|---------:|---------|
| 0% | 100% | Perfect |
| 10% | 42.1% | FAILS |
| 20% | 16.7% | FAILS |
| 30%+ | ~35% | FAILS |

### With EcgPreprocessor (temporal consistency filter)

| Noise Injection | Accuracy | Verdict |
|----------------:|---------:|---------|
| 0% | 100% | Perfect |
| 10% | **75.6%** | DEGRADED |
| 20% | **84.4%** | DEGRADED |
| 30% | **70.0%** | DEGRADED |
| 50% | 63.6% | DEGRADED |
| 70% | 1.9% | FAILS |

**Root cause:** CricketBrain's Gaussian tuning resonates with ANY in-band frequency. The `EcgPreprocessor` mitigates this by requiring temporal consistency (3+ consecutive in-band steps with 2-step gap tolerance), which rejects single-step noise spikes while preserving real QRS bursts (~10ms duration).

**Remaining limitation:** Above 50% noise, even the preprocessor fails because noise spikes become frequent enough to create sustained in-band sequences that pass the temporal filter. This is a fundamental limit of frequency-domain detection without amplitude-level analysis.

---

## 2. Rapid Rhythm Switching

When the rhythm changes every 3 beats (Normal↔Tachycardia), the detector classifies **89% as Irregular** because the RR-interval window (8 beats) always contains a mix of short and long intervals.

**Root cause:** The detector needs 5–8 stable beats to converge on a classification. This is inherent to any interval-based method — not a CricketBrain-specific flaw.

**Implication:** Paroxysmal arrhythmias with rapid onset/offset will be detected as "Irregular" rather than correctly identifying the specific arrhythmia type. This is arguably the correct clinical interpretation — the rhythm IS irregular during transitions.

---

## 3. Frequency-Domain Only

CricketBrain processes **frequency values**, not raw ECG amplitude waveforms. Real-world deployment requires:

1. **R-peak detection** (e.g., Pan-Tompkins algorithm) to extract beat timestamps
2. **RR-interval computation** from adjacent R-peaks
3. **Frequency mapping** from RR intervals to CricketBrain input

This preprocessing pipeline adds latency and complexity. The sub-microsecond CricketBrain latency only applies to the inference step — not the full pipeline.

---

## 4. Rate-Based Classification Only

The detector classifies based on **heart rate** (BPM from RR intervals), NOT on:

- **QRS morphology** (width, amplitude, shape)
- **ST segment** changes (elevation/depression)
- **P-wave** presence/absence
- **T-wave** abnormalities

This means it **cannot detect:**
- Myocardial infarction (ST elevation)
- Bundle branch blocks (wide QRS)
- Atrial fibrillation (absent P-waves with irregular RR)
- Long QT syndrome (QT prolongation)
- Ventricular tachycardia (wide-complex tachycardia)

**CricketBrain detects:** Normal Sinus, Tachycardia, Bradycardia, Irregular.
**CricketBrain does NOT detect:** MI, BBB, AF, VT, LQTS, WPW, or any morphological arrhythmia.

---

## 5. Real-data coverage (v0.4 update)

The synthetic-only blanket caveat that lived here in v0.1/v0.2 is
now partly superseded: v0.4 ships a first real MIT-BIH run on 5
records (100, 105, 200, 217, 232) — 11 375 annotation beats —
with **96.08 % pooled accuracy** and per-class recall {Normal 0.97,
Tachy 0.57, Irregular 0.92, Brady undefined}. See § 0a above.

What is still constrained:

- **Sample is 5 of 48 MIT-BIH records.** AAMI EC57:2012 inter-patient
  evaluation requires a curated record split which v0.4 does not
  enforce. A 10–20 record evaluation with explicit train/test
  patient disjointness is on the v0.5 roadmap milestone.
- **Bradycardia ground truth is missing** in this subset. Cannot
  claim performance on slow rhythms.
- **MIT-BIH only.** No AHA database, no Long-Term ST DB, no Apnea
  ECG, no real wearable traces.
- **Single-channel.** MIT-BIH ships a 2-lead signal but the
  pipeline currently uses one annotation series per record.

---

## 6. CricketBrain vs. Alternatives (Classical DSP, TinyML, DNN)

| Capability | CricketBrain | Pan-Tompkins | Tiny MF-CNN ([src](https://pmc.ncbi.nlm.nih.gov/articles/PMC9919183/)) | Stanford DNN ([src](https://www.nature.com/articles/s41591-018-0268-3)) | Apple Watch AFib |
|-----------|:---:|:---:|:---:|:---:|:---:|
| RAM | **928 B** | < 1 KB | 4-8 KB | GPU-class | proprietary |
| Model size | ~20 KB flash | < 5 KB | **15 KB** | ~10-40 MB | ~1-10 MB |
| Training data | **Zero** | Zero | MIT-BIH inter-patient | 91 k ECGs, 53 k patients | millions |
| Inference latency | **0.126 µs/step** | 1-10 ms | < 1 ms on RPi | 30 s window | 30-60 s |
| Rate classification (N/Tachy/Brady/Irregular) | **Yes** | Yes | Yes | Yes | Partial |
| AAMI 5-class morphology (N/S/V/F/Q) | No | No | **98.18 %** acc, F1 0.92 | Yes | No |
| 12-rhythm class (AF/VT/AVB/...) | No | No | No | **F1 0.837** (beats cardiologist avg 0.780) | AFib only |
| MI / ST-elevation | No | No | No | Partial | No |
| Runs on ATtiny85 ($0.50) | **Yes** | Yes | Tight | No | No |
| Sub-mW average power | **Yes** | Yes | Partial | No | ~3 mW |
| Explainable / deterministic | **Yes** | Yes | Partial | No | No |
| FDA-clearable | Class II plausible | Yes | Yes | Yes (pending) | **Yes (DEN180044)** |

Full sourced breakdown with per-axis numbers, when-to-use-which
matrix, and power-budget maths:
[docs/competitive_analysis.md](competitive_analysis.md).

---

## 7. What CricketBrain CAN Do

Despite these limitations, CricketBrain provides genuine value for:

1. **First-pass screening** — quickly flag abnormal heart rates for review
2. **Embedded deployment** — runs in 928 bytes on any microcontroller
3. **Privacy-preserving** — no data leaves the device
4. **Deterministic** — same input always produces same output
5. **Zero training** — works out of the box, no labeled data needed
6. **Complementary** — can run alongside a deep learning classifier as a fast pre-filter

---

## Conclusion

CricketBrain's cardiac detector is a **rate-based rhythm classifier** optimized for extreme resource constraints. It is NOT a replacement for clinical ECG analysis or deep learning classifiers. Its value lies in the combination of sub-microsecond latency, sub-kilobyte memory, and zero training requirement — properties that no other approach can match simultaneously.

The honest answer is: **use CricketBrain for what it's good at (fast rate screening on embedded devices) and use deep learning for what it's good at (comprehensive morphological analysis).**
