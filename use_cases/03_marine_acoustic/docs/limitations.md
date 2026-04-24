# UC03 Marine Acoustic — Known Limitations

**Date:** 2026-04-24 | **CricketBrain v3.0.0**

---

## 1. Single-Label Output — MITIGATED in v0.2

**Status (v0.1):** The detector reports only the **dominant** channel per
50-step window. Measurements on `marine_stress::E`:

| Mixed Scene | v0.1 Detected | v0.1 Missed |
|-------------|---------------|-------------|
| Fin + Blue | BlueWhale | FinWhale |
| Blue + Humpback | BlueWhale | Humpback |
| Fin + Humpback | Humpback | FinWhale |

**Root cause:** `classify()` takes `argmax` of accumulated energy per
channel. Multiple simultaneous species compete for the decision.

### v0.2 Mitigation: `step_multi()`

v0.2 adds a multi-label API that thresholds every channel independently
and emits a [`MultiLabelDecision`] whose `events` field lists every
active source:

```rust
let mut det = MarineDetector::with_bandwidth(0.20);  // wide tuning
while let Some(decision) = det.step_multi(freq) {
    // decision.events may be [FinWhale, ShipNoise] simultaneously
}
```

**Result on the fin-whale-under-ship scene (`marine_v02` benchmark,
2000 steps):**

| Version | Windows flagging BOTH fin+ship | Coverage |
|---------|-------------------------------:|---------:|
| v0.1 single-label | 0 / 40 | 0 % |
| v0.2a (bw=0.20) multi-label | 40 / 40 | **100 %** |
| v0.2b (bw=0.30) multi-label | 40 / 40 | **100 %** |

**Regression check:** zero false-positive species on 2000 steps of pure
ambient ocean at bandwidth 0.20 (verified in
`v02_wide_bandwidth_still_rejects_truly_ambient` and
`v02_multi_label_single_source_stays_single`).

---

## 2. Boundary Frequencies → Ambient — PARTIALLY MITIGATED in v0.2

**Status (v0.1):** Signals with a dominant frequency between two tuned
channels fall outside every Gaussian tuning curve and are reported as
Ambient.

### v0.2 Mitigation: `with_bandwidth(0.20)`

Each `Neuron` exposes a public `bandwidth` field that controls the
Gaussian selectivity (sigma = bandwidth × eigenfrequency). v0.2 adds
`MarineDetector::with_bandwidth(bw)` which widens every channel's
tuning at construction time. Measured head-to-head on a sustained tone
(`marine_v02` benchmark):

| Input | v0.1 (bw=0.10) | v0.2a (bw=0.20) | v0.2b (bw=0.30) |
|------:|----------------|-----------------|------------------|
| 50 Hz | Ambient | Ambient | Ambient |
| **110 Hz** | Ambient | **Ship Noise** | **Ship Noise** |
| **170 Hz** | Ambient | **Humpback** | **Humpback** |
| 260 Hz | Ambient | Ambient | Humpback |
| 15 Hz | Ambient | Ambient | Fin Whale |
| 80 Hz (exact) | Blue | Blue | Blue |
| 140 Hz (exact) | Ship | Ship | Ship |

### The bandwidth / accuracy trade-off

Wider Gaussians also let out-of-band noise bleed into channels. Bandwidth
sweep on `sample_marine.csv`:

| Bandwidth | CSV accuracy | Boundary recovery |
|-----------|-------------:|-------------------|
| 0.10 (v0.1) | 90 % | none |
| 0.15 | 90 % | partial |
| **0.20 (recommended)** | **90 %** | **110 Hz + 170 Hz** |
| 0.22 | 89 % | + margins |
| 0.25 | 79 % | + margins |
| 0.30 | 75 % | + edges (15 / 260 Hz) |

0.20 is the sweet spot — zero CSV regression and the two
between-channel gaps at 110 / 170 Hz are now assigned to the nearest
species. 0.30 additionally catches out-of-band signals (15 Hz, 260 Hz)
but at a 15 % accuracy cost.

### Remaining gap

The 50 Hz window (between Fin=20 and Blue=80) is still reported as
Ambient even at bandwidth 0.30 — the relative gap (80-20)/20 = 300 %
is too wide for any reasonable Gaussian to bridge. Species in this
range (sei whale downsweeps 40-60 Hz) would need their own dedicated
channel in a 5-channel or 6-channel bank.

---

## 3. No Source Localisation

The detector answers **what** is vocalising, not **where** it is or how
far away it is.

Real passive-acoustic monitoring (PAM) systems use:
- Hydrophone arrays + time-difference-of-arrival for bearing.
- Received-level drop with `20 log10(r)` for range estimation.
- Doppler shift for radial speed (small in water: `c ≈ 1500 m/s`).

**Impact:** Cannot distinguish a single close source from a distant loud
source. Cannot report ship speed or heading. A passing cargo vessel is
detected, but the detector cannot say whether it is 100 m or 5 km away.

**Mitigation:** Combine multiple MarineDetector instances (one per
hydrophone) and perform TDOA in downstream code.

---

## 4. Noise Robustness (GOOD but not perfect)

| Noise % | Accuracy | Verdict |
|--------:|---------:|---------|
| 0 % | 100 % | OK |
| 5 % | 100 % | OK |
| 10 % | 96 % | OK |
| 20 % | 90 % | DEGRADED |
| 30 % | 82 % | DEGRADED |
| 50 % | 76 % | DEGRADED |

The 50-step energy accumulation window averages away short random
transients. Sustained broadband noise (storms, surf, earthquakes)
degrades accuracy above ~20 % contamination.

**Mitigation:** The implemented `set_sea_state()` helper raises the
ambient threshold proportionally and prevents false-positive whale
detections in rough conditions (verified: 100 % Ambient preservation
up to sea state 8 in the stress test).

---

## 5. Synthetic Data Only

All benchmarks in this v0.1.0 release use synthetic frequency streams
derived from the canonical marine-acoustic frequencies. Real MBARI MARS
recordings additionally exhibit:

- Propagation multipath (bottom- and surface-reflected copies).
- Spectral spreading by water-column sound-speed gradients.
- Earthquake T-phase arrivals (common off Monterey Bay).
- Biological chorusing (dawn/dusk fish choruses around 100-300 Hz).
- Impulsive sperm-whale clicks that alias down into our 10-500 Hz band.

Real-data validation is planned for v0.2 once a 24-hour MARS segment is
preprocessed with `python/preprocess.py --wav`.

---

## 6. CricketBrain vs. Established PAM Tools

| Capability | CricketBrain | PAMGuard | Orcasound / DeepAcoustics |
|------------|:------------:|:--------:|:-------------------------:|
| Fixed-frequency species detection | Yes | Yes | Yes |
| Sub-microsecond latency | Yes | No | No |
| Runs on $2 STM32 | Yes | No | No |
| Multi-label simultaneous species | No | Yes | Yes |
| Localisation (TDOA / bearing) | No | Yes | Partial |
| Training data required | No | No | Yes (large) |
| Species repertoire | 4 channels | >50 click/whistle types | Species-specific |
| Power budget | 1 mW @ ESP32 | 10 W laptop | 100 W GPU |

CricketBrain's niche: **continuous edge monitoring on a solar-powered
buoy**, where the ~8 mm² of silicon and milliwatt power budget rule out
PAMGuard or deep-learning approaches. For post-hoc analysis of archived
MARS data, use PAMGuard.
