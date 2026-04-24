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
