# UC03 Marine Acoustic — Benchmark Results

**Date:** 2026-04-24 | **CricketBrain v3.0.0** | **Dataset:** Synthetic (200 windows)

---

## Classification Performance

Run on `data/processed/sample_marine.csv` (5 × 40 = 200 preprocessed windows,
25 detector steps per window).

| Class | TP | FP | Precision | Recall | F1 |
|-------|---:|---:|----------:|-------:|---:|
| Ambient | 19 | 1 | 0.950 | 0.950 | 0.950 |
| Fin Whale (20 Hz) | 19 | 3 | 0.864 | 0.950 | 0.905 |
| Blue Whale (80 Hz) | 17 | 3 | 0.850 | 0.850 | 0.850 |
| Ship Noise (140 Hz) | 17 | 3 | 0.850 | 0.850 | 0.850 |
| Humpback Song (200 Hz) | 18 | 0 | 1.000 | 0.900 | 0.947 |
| **Macro Average** | | | **0.903** | **0.900** | **0.900** |

**Overall Accuracy:** 90/100 = **90.0 %**

The 10 misclassifications cluster at section boundaries — the detector's
50-step energy window needs one full window to adapt to a new dominant
frequency (visible as the two `Conf≈0.53` transitions in the CSV output).

---

## Signal Detection Theory (SDT)

`cargo run --release --example marine_sdt` (200 signal + 200 noise trials
per condition, 500 steps per trial).

| Condition | d' | TPR | FPR | Rating |
|-----------|---:|----:|----:|--------|
| Fin Whale vs Ambient | 6.18 | 1.000 | 0.000 | EXCELLENT |
| Blue Whale vs Ambient | 6.18 | 1.000 | 0.000 | EXCELLENT |
| Ship Noise vs Ambient | 6.18 | 1.000 | 0.000 | EXCELLENT |
| Humpback vs Ambient | 6.18 | 1.000 | 0.000 | EXCELLENT |
| Ambient vs Ship Noise | 6.18 | 1.000 | 0.000 | EXCELLENT |

Wilson 95 % CI: TPR [0.981, 1.000] · FPR [0.000, 0.019].

---

## Latency & Throughput

`cargo run --release --example marine_latency` (100 runs per condition).

| Condition | First Detection | Speed |
|-----------|----------------:|------:|
| Ambient | 49 ms | 0.130 µs/step |
| Fin Whale (20 Hz) | 49 ms | 0.179 µs/step |
| Blue Whale (80 Hz) | 49 ms | 0.242 µs/step |
| Ship Noise (140 Hz) | 49 ms | 0.241 µs/step |
| Humpback (200 Hz) | 49 ms | 0.276 µs/step |

Throughput peak ≈ 7.7 M steps/sec. At the MBARI MARS decimated rate of
2 kHz, the detector consumes < 0.06 % of a single CPU core.

---

## Memory Footprint

| Component | Value |
|-----------|------:|
| ResonatorBank | 3,712 bytes |
| Neurons | 20 (4 channels × 5) |
| Bytes/neuron | 185.6 |
| MarineDetector struct | 88 bytes |
| **Total** | **3,800 bytes** |

| Target Platform | SRAM | Fits? |
|-----------------|-----:|:-----:|
| ATtiny85 | 512 B | No |
| Arduino Uno | 2 KB | No |
| STM32F0 | 4 KB | **Yes** |
| ESP32 | 520 KB | **Yes** |
| Smart-Buoy budget (16 KB) | 16 KB | **Yes (4× margin)** |

---

## Stress-Test Highlights

`cargo run --release --example marine_stress`.

### A) Noise Robustness

Random frequency spikes injected into a 500-step ship-passage signal.

| Noise % | Accuracy | Verdict |
|--------:|---------:|---------|
| 0 % | 100 % | OK |
| 5 % | 100 % | OK |
| 10 % | 96 % | OK |
| 20 % | 90 % | DEGRADED |
| 30 % | 82 % | DEGRADED |
| 50 % | 76 % | DEGRADED |

### B) Ship Transits

A cargo vessel sailing past the hydrophone (triangular presence profile
centred on closest point of approach).

| Transit length | Ship windows | Total windows | Ship ratio |
|---:|---:|---:|---:|
| 500 | 7 | 10 | 70.0 % |
| 1000 | 11 | 20 | 55.0 % |
| 1500 | 20 | 30 | 66.7 % |
| 2000 | 25 | 40 | 62.5 % |
| 3000 | 39 | 60 | 65.0 % |
| 5000 | 59 | 100 | 59.0 % |

The CPA portion of every transit is flagged as `ShipNoise`; the approach
and recede tails contribute `Ambient` windows, which is the physically
correct answer.

### C) Whale Under Ship

Fin-whale pulses interleaved with a ship passage (`fin_whale_under_ship`,
2000 steps). The detector produces 32 FinWhale and 8 ShipNoise windows —
the endangered-species signal survives masking by anthropogenic noise.

### D) Sea State

Ambient noise with the detector pre-configured for Douglas sea states 0 / 2 /
4 / 6 / 8. All five runs produce **100 % Ambient** classifications (zero
false alarms). The `set_sea_state()` threshold scaling absorbs the elevated
broadband background without triggering spurious whale detections.

### E-G)

Simultaneous-species mixing picks the dominant source (no multi-label).
Boundary frequencies between two channels are reported as `Ambient`
because the Gaussian tuning is strict. Rapid scene changes require one
full 50-step window to adapt (visible `Conf ≈ 0.6` on transition windows).

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
cd use_cases/03_marine_acoustic

cargo run --release -- --csv data/processed/sample_marine.csv
cargo run --release -- --ship-transit
cargo run --release --example marine_sdt
cargo run --release --example marine_latency
cargo run --release --example marine_memory
cargo run --release --example marine_stress

pip install -r python/requirements.txt
python python/preprocess.py --synthetic
python python/plot_results.py
python python/evaluate.py
```
