# CricketBrain Use Cases — MASTER PLAN

> This file is the single source of architectural knowledge for all 10 use cases.
> Updated after every run. Read this FIRST before any work.

---

## 1. Project Architecture Summary

### Core Engine (src/brain.rs)
- `CricketBrain::new(BrainConfig) -> Result<Self, CricketError>`
- `CricketBrain::step(input_freq: f32) -> f32` — one timestep, returns spike amplitude
- `CricketBrain::step_batch(inputs: &[f32]) -> Vec<f32>` — batch processing
- `CricketBrain::step_with_telemetry(input_freq, &mut T) -> f32` — with event hooks
- `CricketBrain::reset()` — zero all state
- `CricketBrain::enable_stdp(StdpConfig)` / `enable_homeostasis(HomeostasisConfig)`
- `CricketBrain::snapshot()` / `restore_from_snapshot()` — serializable state

### BrainConfig (11 fields)
| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| n_neurons | usize | 5 | Network size |
| min_freq | f32 | 4000.0 | Eigenfrequency lower bound |
| max_freq | f32 | 5000.0 | Eigenfrequency upper bound |
| k_connections | Option<usize> | None (= n*3) | Synapse count |
| sample_rate_hz | u32 | 1000 | 1 ms timesteps |
| min_activation_threshold | f32 | 0.0 | Noise floor |
| adaptive_sensitivity | bool | false | AGC |
| agc_rate | f32 | 0.01 | AGC speed |
| seed | u64 | 0xC0DEC0DE5EED | Deterministic RNG |
| privacy_mode | bool | false | HIPAA/GDPR coarsening |
| noise_level | f32 | 0.0 | Stochastic noise injection |

### Sequence Predictor (src/sequence.rs)
- `SequencePredictor::new(vocab, config) -> Result`
- `register_pattern(name, labels)` — define temporal patterns
- `step(input_freq) -> Vec<f32>` — per-channel outputs
- `predict() -> Option<Prediction>` — best match with confidence

### Telemetry (crates/core/src/logger.rs)
- `TelemetryEvent::Spike { neuron_id, timestamp }`
- `TelemetryEvent::SequenceMatched { pattern_id, confidence, snr, jitter, tolerance }`
- `TelemetryEvent::SystemOverload { entropy, active_neurons, total_neurons }`

### Cross-Language Bindings
| Language | Crate | API |
|----------|-------|-----|
| Python | crates/python | `Brain(config?)`, `step(freq)`, `step_batch([freq])`, `reset()` |
| C/C++ | crates/ffi | `brain_new(&h, n, min, max)`, `brain_step(h, freq, &out)`, `brain_free(h)` |
| WASM/JS | crates/wasm | `new Brain(seed?)`, `step(freq)`, `drainTelemetry()`, `latestPrediction()` |

---

## 2. Global Metrics (from metrics.json)

| Metric | Value | Source |
|--------|-------|--------|
| Version | 3.0.0 | Cargo.toml |
| RAM | 928 bytes | `memory_usage_bytes()` measured |
| Neurons | 5 | Canonical circuit |
| Synapses | 6 | Canonical circuit |
| Tests | 122 | `cargo test --workspace` |
| Latency | 0.175 µs/step | SynOPS benchmark |
| Throughput | 10.7M steps/sec | SynOPS benchmark |
| Checksum | FNV-1a | src/brain.rs:856 |
| License | AGPL-3.0 | Cargo.toml |
| MSRV | Rust 1.75 | Cargo.toml |

---

## 3. License Matrix

| Binding | License | Commercial Use |
|---------|---------|----------------|
| Core Rust | AGPL-3.0 | Requires paid license for proprietary |
| Python (PyO3) | AGPL-3.0 | Same |
| C FFI | AGPL-3.0 | Same |
| WASM | AGPL-3.0 | Same |
| Startup (<1M EUR) | Perpetual, 1 product, 5 devs | Contact belkis.aslani@gmail.com |
| Professional | Unlimited products/devs, 72h SLA | Contact |
| Enterprise (>50M EUR) | Unlimited, 24h SLA, custom opt | Contact |
| OEM/Embedded | Per-unit royalty, binary rights | Contact |

**For use cases:** All research/demo code is AGPL-3.0. Datasets have their own licenses (see per-UC metrics.json entries).

---

## 4. Build Plan — All 10 Use Cases

Each use case follows the same 10-run progression:

| Run | Deliverable | Validates |
|-----|-------------|-----------|
| 1 | Scaffold: Cargo.toml, README, CLAUDE.md | Structure compiles |
| 2 | Data pipeline: download, parse, preprocess | Raw → processed data |
| 3 | Core detector: Rust impl using CricketBrain API | Basic detection works |
| 4 | Benchmark suite: SDT, latency, memory | Quantitative results |
| 5 | Python binding: analysis scripts, plots | Cross-language works |
| 6 | Stress test: adversarial conditions | Honest limits documented |
| 7 | Website demo page: interactive visualization | End-user experience |
| 8 | Documentation: full README, API docs | Complete for users |
| 9 | CI integration: build + test in workflow | Automated validation |
| 10 | Metrics update: write results to metrics.json | Numbers flow everywhere |

### Per-UC Dataset & Signal Info

| UC | Dataset | License | Signal Hz | Status |
|----|---------|---------|-----------|--------|
| 01 Cardiac | MIT-BIH Arrhythmia | ODC-By | 360 | **COMPLETE** (10/10) |
| 02 Bearings | CWRU Bearing | Public Domain | 12,000 | **COMPLETE** (10/10) |
| 03 Marine | MBARI MARS | CC BY 4.0 | 256,000 | **COMPLETE** (10/10) |
| 04 Grid | EPFL Smart Grid | CC BY 4.0 | 50 | Planned |
| 05 Network | KDD Cup 1999 | Public Domain | — | Planned |
| 06 Agriculture | BioAcoustica | CC BY 4.0 | varies | Planned |
| 07 Vehicle | UrbanSound8K | CC BY 4.0 | varies | Planned |
| 08 Hearing | DNS Challenge | CC BY 4.0 | varies | Planned |
| 09 QC | MIMII | CC BY 4.0 | varies | Planned |
| 10 Space | NASA SMAP/MSL | Public Domain | varies | Planned |

---

## 5. Existing Code to Reuse

| Asset | Path | Reuse For |
|-------|------|-----------|
| ECG demo | examples/sentinel_ecg_monitor.rs | UC01 base logic |
| Baselines | examples/baselines.rs | Comparison framework (MF, Goertzel, IIR) |
| SDT benchmark | benchmarks/sdt_benchmark.rs | d' / AUC evaluation |
| Stress test | benchmarks/stress_test_benchmark.rs | Adversarial testing pattern |
| SynOPS bench | benchmarks/synops_benchmark.rs | Efficiency metrics |
| Ablation | examples/ablation_study.rs | Component contribution analysis |

---

## 6. Checklists

### Done
- [x] use_cases/ directory structure created (10 UCs + shared)
- [x] metrics.json with global + per-UC data
- [x] inject_metrics.py script
- [x] README.template.md and page.template.html
- [x] UC01 SOURCES.md (MIT-BIH dataset)
- [x] MASTER_PLAN.md (this file)
- [x] **UC01 Cardiac Arrhythmia — ALL 10 RUNS COMPLETE** (2026-04-10)
  - Run 1: Scaffold (Cargo.toml, src/, 7 tests)
  - Run 2: Data pipeline (Python download/preprocess, CSV I/O, 9 tests)
  - Run 3: CSV classification + confusion matrix (11 tests, 92.5% accuracy)
  - Run 4: Benchmark suite (SDT d'=6.18, latency 0.126µs, memory 928B)
  - Run 5: Python evaluation (F1=0.962, 3 PNG plots)
  - Run 6: Stress test (noise fails >10%, boundary ±1BPM works)
  - Run 7: Website demo page (cardiac.html on main site)
  - Run 8: Full documentation (README, API reference)
  - Run 9: CI workflow (uc01-cardiac.yml, all steps verified)
  - Run 10: Metrics finalization (metrics.json updated with real values)

- [x] **UC02 Predictive Maintenance — ALL 10 RUNS COMPLETE** (2026-04-10)
  - 4-channel ResonatorBank (FTF / BSF / BPFO / BPFI), 93.0% accuracy, d'=6.18
  - 20 neurons / 3,712 bytes RAM, 0.13-0.26 µs/step, STM32F0 ready
- [x] **UC03 Marine Acoustic — ALL 10 RUNS COMPLETE** (2026-04-24)
  - 4-channel ResonatorBank (FIN 20 Hz / BLUE 80 Hz / SHIP 140 Hz / HUMP 200 Hz)
  - 90.0% accuracy, d'=6.18, 20 tests passing
  - 20 neurons / 3,712 bytes RAM, 0.13-0.28 µs/step, smart-buoy ready
  - Ship-transit tests, whale-under-ship scenario, sea-state compensation

### Planned
- [ ] UC04 Power Grid: Runs 1-10
- [ ] UC05 Network Intrusion: Runs 1-10
- [ ] UC06 Precision Agriculture: Runs 1-10
- [ ] UC07 Autonomous Vehicle: Runs 1-10
- [ ] UC08 Hearing Aid: Runs 1-10
- [ ] UC09 Quality Control: Runs 1-10
- [ ] UC10 Space Systems: Runs 1-10

---

---

## 7. v0.2 Priority Backlog — Real-Data Validation

All three completed use cases currently benchmark on **synthetic
signals**. The next-highest-value work is not a fourth use case but
**real-data validation** of the existing three, in this order:

1. **UC02 Bearings — real CWRU `.mat` files** (easiest; dataset is
   already used by the Python preprocess pipeline, just needs the
   end-to-end run). Highest credibility-per-hour payoff.
2. **UC03 Marine — real MBARI MARS hydrophone segments** (AWS Open
   Data). Harder because labelling / preprocessing requires
   domain knowledge.
3. **UC01 Cardiac — real MIT-BIH records** (most regulatory care
   required; frame as "rate-based triage demo", not clinical).

Each validation should add a `docs/real_data_results.md` alongside the
existing synthetic `docs/results.md` so the distinction stays visible.
Until real-data rows exist, *all accuracy claims carry a "synthetic-
window accuracy" qualifier*.

---

*Last updated: 2026-04-24 — UC03 v0.2 complete (10/10 runs + v0.2 API);
3 of 10 use cases done; real-data validation is the next v0.2
priority across all three.*
