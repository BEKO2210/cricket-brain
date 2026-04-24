# UC03 Marine Acoustic — Known Limitations

**Date:** 2026-04-24 | **CricketBrain v3.0.0**

---

## 1. Single-Label Output

The detector reports only the **dominant** channel per 50-step window.
Measurements on `marine_stress::E`:

| Mixed Scene | Detected | Missed |
|-------------|----------|--------|
| Fin + Blue | BlueWhale | FinWhale |
| Blue + Humpback | BlueWhale | Humpback |
| Fin + Humpback | Humpback | FinWhale |

**Root cause:** `classify()` takes `argmax` of accumulated energy per
channel. Multiple simultaneous species compete for the decision.

**Impact:** During biologically rich encounters (fin whales and blue
whales sharing a foraging ground; mother-calf humpback escorts calling
while a cargo vessel transits), only the loudest source is reported.

**Mitigation:** Threshold every channel independently and emit a
multi-label vector. This would require a small API change (return a
`Vec<AcousticEvent>` instead of a single enum) and is planned for v0.2.

---

## 2. Boundary Frequencies → Ambient

Signals with a dominant frequency between two tuned channels fall outside
every Gaussian tuning curve and are reported as Ambient. From
`marine_stress::F`:

| Input | Between | Decision |
|------:|---------|----------|
| 50 Hz | Fin (20) and Blue (80) | Ambient |
| 110 Hz | Blue (80) and Ship (140) | Ambient |
| 170 Hz | Ship (140) and Hump (200) | Ambient |
| 15 Hz | below Fin (20) | Ambient |
| 260 Hz | above Hump (200) | Ambient |
| 80 Hz | exact Blue | BlueWhale |

**Root cause:** `TokenVocabulary::new(&["FIN", "BLUE", "SHIP", "HUMP"],
20.0, 200.0)` places four Gaussian resonators at 20 / 80 / 140 / 200 Hz.
Anything outside a ±10 % tuning envelope misses all four.

**Impact:** Species with frequencies between the four canonical channels
(e.g. minke whale "boing" ~1.3 kHz; sei whale downsweeps 40-60 Hz) are
invisible to this detector.

**Mitigation:** Spawn a wider ResonatorBank with more channels
(up to the 27-token marine vocabulary described in the MASTER_PLAN's
UC03 `key_advantage`) at the cost of ~928 bytes per additional 5-neuron
circuit.

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
