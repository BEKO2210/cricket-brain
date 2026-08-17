# UC03 Marine Acoustic — CLAUDE Build Plan

> Local plan file for Use Case 03. Updated after every run.

---

## 1. Overview

Passive-acoustic monitoring (PAM) of a low-frequency ocean soundscape.
The MBARI MARS cabled observatory in Monterey Bay streams 256 kHz
hydrophone audio; the 10-500 Hz baleen-whale band is routinely mined
for fin-whale 20-Hz pulses, blue-whale A-calls, humpback song, and
cargo-ship radiated noise.

CricketBrain classifies each 128 ms FFT window into one of five
`AcousticEvent` values using a 4-channel Gaussian-tuned resonator bank.
No training data, 3,800 bytes of RAM, runs on a solar-powered smart buoy.

---

## 2. Dataset: MBARI MARS Cabled Observatory

| Field | Value |
|-------|-------|
| Source | Monterey Bay Aquarium Research Institute |
| URL | https://www.mbari.org/technology/mars/ |
| Data portal | https://docs.mbari.org/pacific-sound/ |
| License | CC BY 4.0 |
| Location | Monterey Bay, CA — 891 m depth |
| Sampling | 256 kHz broadband, 2 kHz decimated for baleen-whale work |
| Format | FLAC (lossless), 24-bit |
| Coverage | Continuous since 2015 (~2 PB available) |

### Key Frequencies (10-500 Hz band)
- FIN — fin whale 20-Hz stereotyped pulse: **20 Hz**
- BLUE — blue whale A-call (NE Pacific): **80 Hz**
- SHIP — cargo-ship propeller cavitation peak: **140 Hz**
- HUMP — humpback song mid-band unit: **200 Hz**

---

## 3. CricketBrain Approach

Use a **ResonatorBank** with four Gaussian-tuned channels distributed
evenly in [20, 200] Hz. Each channel is a 5-neuron circuit. When a
source frequency is sustained, the matching coincidence gate fires →
species or ship noise detected.

```rust
let vocab = TokenVocabulary::new(
    &["FIN", "BLUE", "SHIP", "HUMP"],
    20.0,   // min freq
    200.0,  // max freq
);
let mut bank = ResonatorBank::new(&vocab);
let outputs = bank.step(input_freq);
// outputs[0] > 0 → fin whale
// outputs[1] > 0 → blue whale
// outputs[2] > 0 → ship noise
// outputs[3] > 0 → humpback song
```

Classification in `MarineDetector` accumulates channel energy over a
50-step window, returns `Ambient` when the total falls below
`ambient_threshold` (scaled by Douglas sea state), otherwise emits the
`argmax` channel as an `AcousticEvent`.

---

## 4. Ten-Run Plan

| Run | Deliverable | Status |
|-----|-------------|--------|
| 1 | Scaffold (Cargo.toml, src/, SOURCES, README) | DONE 2026-04-24 |
| 2 | Data pipeline (Python STFT, CSV I/O) | DONE 2026-04-24 |
| 3 | Core detector (CSV classify + ConfusionMatrix) | DONE 2026-04-24 |
| 4 | Benchmark suite (SDT / latency / memory) | DONE 2026-04-24 |
| 5 | Python analysis (evaluate.py + 3 plots) | DONE 2026-04-24 |
| 6 | Stress test (adversarial, ship transits) | DONE 2026-04-24 |
| 7 | Website demo (`marine.html`) | DONE 2026-04-24 |
| 8 | Full README + docs/api.md | DONE 2026-04-24 |
| 9 | CI workflow (`ci.yml`) | DONE 2026-04-24 |
| 10 | Metrics finalization (shared/metrics.json) | DONE 2026-04-24 |

---

## 5. Measured Results (2026-04-24)

| Metric | Value |
|--------|-------|
| Accuracy (CSV) | 90.0 % (90/100) |
| Macro F1 | 0.900 |
| d' (SDT) | 6.18 (all 5 conditions EXCELLENT) |
| Latency | 0.130-0.276 µs/step |
| First decision | 49 ms (50 steps × 1 ms) |
| RAM | 3,712 bytes (20 neurons) |
| Detector struct | 88 bytes |
| Noise tolerance | 100 % @ ≤10 %, 90 % @ 20 % |
| Ship transit CPA | correctly flagged in every length (500-5000 steps) |
| Sea state 0-8 | 100 % Ambient preservation |
| Tests passing | 20 / 20 |

---

## 6. Run Status

| Run | Status | Date | Notes |
|-----|--------|------|-------|
| 1 | DONE | 2026-04-24 | Cargo.toml, src/, 20/20 tests pass |
| 2 | DONE | 2026-04-24 | STFT pipeline, CSV I/O, sample_marine.csv |
| 3 | DONE | 2026-04-24 | 90 % CSV accuracy, 5-class ConfusionMatrix |
| 4 | DONE | 2026-04-24 | d'=6.18 all EXCELLENT, 0.13-0.28 µs/step, 3,712 B |
| 5 | DONE | 2026-04-24 | evaluate.py, plot_results.py (3 plots) |
| 6 | DONE | 2026-04-24 | Ship transits, whale-under-ship, sea state, boundary |
| 7 | DONE | 2026-04-24 | website/pages/marine.html |
| 8 | DONE | 2026-04-24 | Full README + docs/api.md + docs/limitations.md |
| 9 | DONE | 2026-04-24 | ci.yml — build/test/benchmarks/python-syntax |
| 10 | DONE | 2026-04-24 | shared/metrics.json updated with measured values |

---

## 7. v0.2 Additions (2026-04-24)

Backwards-compatible additive API addressing the two biggest v0.1
limitations (single-label + boundary frequencies).

### New API

```rust
MarineDetector::with_bandwidth(0.20)       // wide-tuning constructor
det.set_bandwidth(0.20)                    // runtime tuning
det.set_channel_threshold(0.03)            // per-channel multi-label bar
det.step_multi(freq) -> Option<MultiLabelDecision>
```

### Measured Impact (`cargo run --release --example marine_v02`)

- **Boundary recovery:** 110 Hz (was Ambient → now ShipNoise), 170 Hz
  (was Ambient → now Humpback) at recommended bandwidth 0.20.
- **Multi-label:** fin-whale-under-ship scene — 0 / 40 → **40 / 40**
  windows flagging both species simultaneously (100 %).
- **No regression:** CSV accuracy stays at 90 % at bandwidth 0.20; zero
  false-positive species on 2000-step pure ambient.

### Bandwidth / accuracy trade-off

| BW | CSV acc | 110 Hz | 170 Hz | 260 Hz | 15 Hz |
|----|---:|---|---|---|---|
| 0.10 (v0.1) | 90 % | Ambient | Ambient | Ambient | Ambient |
| **0.20 (v0.2 rec.)** | **90 %** | **Ship** | **Hump** | Ambient | Ambient |
| 0.30 | 75 % | Ship | Hump | Hump | Fin |

### Tests (5 new)

- `v02_wide_bandwidth_catches_boundary_110hz`
- `v02_wide_bandwidth_catches_boundary_170hz`
- `v02_wide_bandwidth_still_rejects_truly_ambient`
- `v02_multi_label_reports_whale_and_ship_together`
- `v02_multi_label_single_source_stays_single`

Total tests: 20 (v0.1) + 5 (v0.2) = **25 / 25 passing**.

---

## 8. License

- Source code: AGPL-3.0-only
- MBARI MARS data: CC BY 4.0 (attribution required)
