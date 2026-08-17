# UC04 Power Grid — Benchmark Results

**Date:** 2026-04-24 | **CricketBrain v3.0.0** | **Dataset:** Synthetic
PMU-style stream (200 windows)

All numbers below are measured on **synthetic 50 Hz grid signals** —
not on real EPFL PMU recordings. Real-data validation is pending.

---

## Classification Performance

`cargo run --release -- --csv data/processed/sample_grid.csv` (200
synthetic windows × 25 detector steps).

| Class | TP | FP | Precision | Recall | F1 |
|-------|---:|---:|----------:|-------:|---:|
| Outage | 19 | 1 | 0.950 | 0.950 | 0.950 |
| Nominal (50 Hz) | 19 | 3 | 0.864 | 0.950 | 0.905 |
| 2nd Harmonic (100 Hz) | 17 | 3 | 0.850 | 0.850 | 0.850 |
| 3rd Harmonic (150 Hz) | 17 | 3 | 0.850 | 0.850 | 0.850 |
| 4th Harmonic (200 Hz) | 18 | 0 | 1.000 | 0.900 | 0.947 |
| **Macro Average** | | | **0.903** | **0.900** | **0.900** |

**Synthetic-window accuracy:** 90 / 100 = **90.0 %**

The 10 misclassifications cluster at section boundaries — the detector
needs one full 50-step window to adapt to a new dominant frequency.

---

## Signal Detection Theory (SDT)

`cargo run --release --example grid_sdt` (200 signal + 200 noise trials
per condition, 500 steps per trial).

| Condition | d' | TPR | FPR | Rating |
|-----------|---:|----:|----:|--------|
| Nominal vs Outage | 6.18 | 1.000 | 0.000 | EXCELLENT |
| 2nd Harmonic vs Nominal | 6.18 | 1.000 | 0.000 | EXCELLENT |
| 3rd Harmonic vs Nominal | 6.18 | 1.000 | 0.000 | EXCELLENT |
| 4th Harmonic vs Nominal | 6.18 | 1.000 | 0.000 | EXCELLENT |
| Outage vs Nominal | 6.18 | 1.000 | 0.000 | EXCELLENT |

Wilson 95 % CI: TPR [0.981, 1.000] · FPR [0.000, 0.019].

**d' convention.** d' uses the **log-linear correction** for ceiling
hit-rates and floor false-alarm rates (hits clipped to
`[0.5/n, 1 − 0.5/n]` before the inverse-normal transform; Hautus 1995).
Without the correction, perfect TPR = 1.000 / FPR = 0.000 cells would
yield an undefined / infinite d'. The 6.18 value is the finite ceiling
for n = 200 trials/class.

---

## Latency & Throughput

`cargo run --release --example grid_latency` (100 runs per condition).

| Condition | First Detection | Speed |
|-----------|----------------:|------:|
| Outage | 49 ms | 0.127 µs/step |
| Nominal (50 Hz) | 49 ms | 0.195 µs/step |
| 2nd Harmonic (100 Hz) | 49 ms | 0.274 µs/step |
| 3rd Harmonic (150 Hz) | 49 ms | 0.230 µs/step |
| 4th Harmonic (200 Hz) | 49 ms | 0.339 µs/step |

Throughput peak ≈ 7.9 M steps/sec. At an EPFL PMU 50 frame/s reporting
rate, the detector consumes < 0.001 % of a single CPU core.

---

## Memory Footprint

| Component | Value |
|-----------|------:|
| ResonatorBank | 3,712 bytes |
| Neurons | 20 (4 channels × 5) |
| Bytes/neuron | 185.6 |
| GridDetector struct | 88 bytes |
| **Total** | **3,800 bytes** |

| Target Platform | SRAM | Fits? |
|-----------------|-----:|:-----:|
| ATtiny85 | 512 B | No |
| Arduino Uno | 2 KB | No |
| STM32F0 | 4 KB | **Yes** |
| ESP32 | 520 KB | **Yes** |
| Substation gateway 64 KB | 64 KB | **Yes (15× margin)** |
| PMU (e.g. SEL-487E) 1 MB | 1 MB | **Yes (250× margin)** |

---

## Stress-Test Highlights

`cargo run --release --example grid_stress`.

### A) Noise Robustness

Random frequency spikes injected into a 500-step 3rd-harmonic signal.

| Synthetic noise % | Accuracy | Verdict |
|-----:|---------:|---------|
| 0 % | 100 % | OK |
| 5 % | 100 % | OK |
| 10 % | 100 % | OK |
| 20 % | 100 % | OK |
| 30 % | 100 % | OK |
| 50 % | 98 % | OK |

The 50-step energy accumulation window absorbs random transients
extremely well. **Caveat:** "100 %" is on synthetic random-spike noise,
not on real PMU recordings with sustained broadband interference.

### B) Factory-Startup Transients

Nominal grid → 3rd-harmonic burst (variable length) → recovery to
nominal:

| Disturb steps | H3 windows | Total | H3 ratio |
|---:|---:|---:|---:|
| 200 | 4 | 30 | 13.3 % |
| 400 | 8 | 30 | 26.7 % |
| 600 | 12 | 30 | 40.0 % |
| 800 | 16 | 30 | 53.3 % |
| 1200 | 24 | 36 | 66.7 % |

The ratio of H3 windows scales linearly with the disturbance length.

### C) Rolling Brownouts

| Dips | Dip length | Outage windows | Nominal | Outage % |
|---:|---:|---:|---:|---:|
| 2 | 60 | 2 | 38 | 5.0 % |
| 4 | 80 | 4 | 36 | 10.0 % |
| 6 | 100 | 6 | 34 | 15.0 % |
| 10 | 120 | 10 | 30 | 25.0 % |

Each scheduled dip produces exactly one Outage window — clean
separation from the Nominal majority.

### D-F)

Simultaneous harmonics (single-label only — picks dominant; v0.2
`step_multi` flags both). Off-nominal fundamental (49.5-50.5 Hz) all
classify as `Nominal` — this triage detector reports CATEGORY, not
exact frequency. For ±0.1 Hz monitoring use a dedicated PMU.

---

## Visualisations

### Event Timeline
![Event Timeline](event_timeline.png)

### Confusion Matrix
![Confusion Matrix](confusion_matrix.png)

### Confidence Distribution
![Confidence Distribution](confidence_dist.png)

---

## Reproduction

```bash
cd use_cases/04_power_grid

cargo run --release -- --csv data/processed/sample_grid.csv
cargo run --release -- --factory
cargo run --release -- --brownout
cargo run --release --example grid_sdt
cargo run --release --example grid_latency
cargo run --release --example grid_memory
cargo run --release --example grid_stress

pip install -r python/requirements.txt
python python/preprocess.py --synthetic
python python/plot_results.py
python python/evaluate.py
```
