# UC01 Cardiac Arrhythmia — Known Limitations

**Date:** 2026-04-10 | **CricketBrain v3.0.0**

> Scientific integrity requires transparent documentation of failure modes.

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

## 5. Synthetic Data Only

All benchmarks were run on **synthetic P-QRS-T waveforms** with:
- Perfect timing (zero jitter)
- Uniform RR intervals within each segment
- Clean frequency transitions
- No physiological variability

Real ECG data (MIT-BIH) has:
- Natural beat-to-beat variability (HRV)
- Ectopic beats (PVCs, PACs)
- Motion artifacts
- Baseline wander
- Electrode noise
- Paced rhythms

The 92.5% accuracy on synthetic data is an **upper bound** — real-world accuracy will be lower.

---

## 6. What CricketBrain CANNOT Do vs. Deep Learning

| Capability | CricketBrain | Deep Learning (e.g., InceptionTime) |
|-----------|:---:|:---:|
| Rate classification (N/Tachy/Brady) | Yes | Yes |
| Morphological analysis | No | Yes |
| Multi-class arrhythmia (AAMI) | No | Yes (5+ classes) |
| Works without training data | Yes | No |
| Works on microcontroller | Yes | No |
| Sub-microsecond latency | Yes | No |
| Explainable decisions | Yes | Partially |
| Real-time on wearable | Yes | Sometimes |
| Handles noise robustly | No (>10%) | Yes (trained on noisy data) |
| Detects MI/STEMI | No | Yes |

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
