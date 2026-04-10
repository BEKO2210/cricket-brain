# CricketBrain Application: Cardiac Arrhythmia Pre-Screening

> **Status:** In Development | **CricketBrain v3.0.0** | **License:** AGPL-3.0

> **NOT A MEDICAL DEVICE.** This application is a research prototype for educational
> and experimental purposes only. Do not use for clinical diagnosis, patient monitoring,
> or any safety-critical decision-making without appropriate regulatory approval
> (FDA 510(k), CE MDR Class IIa, etc.). See [disclaimer](#medical-disclaimer) below.

---

## Overview

Continuous ECG monitoring on wearables requires detecting irregular heartbeat
patterns (atrial fibrillation, tachycardia, bradycardia) in real-time.
CricketBrain's 5-neuron coincidence detection circuit provides sub-microsecond
rhythm classification in 928 bytes of RAM — no cloud, no training data, no
battery drain.

**Market Size:** $50B | **Key Advantage:** 928 bytes RAM, Privacy-first, bestehende Demo

## CricketBrain Properties

| Property | Value |
|----------|-------|
| RAM Footprint | 928 bytes |
| Latency | 0.175 µs/step |
| Throughput | 10.7M steps/sec |
| Neurons | 5 |
| Synapses | 6 |
| Checksum | FNV-1a |

## Dataset

| Field | Value |
|-------|-------|
| Primary Dataset | MIT-BIH Arrhythmia Database |
| License | Open Data Commons Attribution |
| URL | https://physionet.org/content/mitdb/1.0.0/ |
| Signal Rate | 360 Hz |
| Records | 48 × 30 min two-channel ambulatory ECG |
| Annotations | ~110,000 beat labels by 2+ cardiologists |

## Results

| Metric | Value |
|--------|-------|
| Accuracy | TBD |
| Latency | TBD ms |
| False Positive Rate | TBD |

> Results marked **TBD** will be filled in after benchmarking against MIT-BIH dataset.

## Quick Start

```bash
cd use_cases/01_cardiac_arrhythmia
cargo build
cargo run
cargo test
```

### Expected Output

```
=== CricketBrain Cardiac Arrhythmia Pre-Screening ===
--- Normal Sinus Rhythm (5 cycles, expected ~73 BPM) ---
  Beat 1: Normal Sinus | BPM=73 | Confidence=0.85
  ...
--- Tachycardia (5 cycles, expected ~188 BPM) ---
  Beat 1: Tachycardia | BPM=188 | Confidence=0.90
  ...
```

## Project Structure

```
├── data/
│   ├── raw/           # Original MIT-BIH files (not committed)
│   ├── processed/     # Preprocessed R-R intervals
│   └── SOURCES.md     # Dataset provenance and download instructions
├── src/
│   ├── lib.rs         # Module exports
│   ├── detector.rs    # CardiacDetector — rhythm classification engine
│   ├── ecg_signal.rs  # Synthetic ECG waveform generation
│   └── main.rs        # Demo binary
├── tests/             # Integration tests
├── benchmarks/        # SDT, latency, memory benchmarks
├── python/            # Analysis scripts (ROC curves, confusion matrix)
├── docs/              # Architecture, API reference, limitations
└── website/           # Interactive web demo
```

## Architecture

```
  ECG Signal (freq/ms) ──→ CricketBrain (5N/6S) ──→ Spike Output
                                                        │
                                                   RR Interval
                                                    Tracking
                                                        │
                                                   Classification
                                                   ┌────┴────┐
                                              Normal  Tachy  Brady  Irregular
```

The `CardiacDetector` wraps a standard CricketBrain instance tuned to the QRS
complex frequency (4500 Hz). When the coincidence gate fires, the detector
records a "beat" timestamp. The sequence of inter-beat intervals (RR intervals)
is then analyzed to classify rhythm:

- **Normal Sinus:** 60–100 BPM, low variability (CV < 0.3)
- **Tachycardia:** > 100 BPM
- **Bradycardia:** < 60 BPM
- **Irregular:** CV > 0.3 (variable intervals)

## Medical Disclaimer

> **THIS SOFTWARE IS NOT A MEDICAL DEVICE.**
>
> It has not been validated for clinical use, has not received regulatory
> clearance (FDA, CE, or equivalent), and must NOT be used for:
> - Clinical diagnosis or treatment decisions
> - Patient monitoring in healthcare settings
> - Any safety-critical or life-sustaining purpose
>
> This is a **research prototype** demonstrating neuromorphic signal processing
> concepts. Any clinical application requires independent validation, regulatory
> approval, and clinical-grade hardware.
>
> The authors accept no liability for any use of this software in medical contexts.

## References

- [CricketBrain Research Whitepaper](../../RESEARCH_WHITEPAPER.md)
- [USE_CASES.md — Cardiac Arrhythmia](../../USE_CASES.md#1-cardiac-arrhythmia-pre-screening-on-wearables)
- [Existing Demo](../../examples/sentinel_ecg_monitor.rs)
- [MIT-BIH Dataset](https://physionet.org/content/mitdb/1.0.0/)
- [Contributing Guide](../../CONTRIBUTING.md)

## Metrics Source

All metrics in this document are sourced from
[`use_cases/shared/metrics.json`](../shared/metrics.json).
Run `python use_cases/shared/scripts/inject_metrics.py` to update.
