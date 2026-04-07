# Changelog

All notable changes to CricketBrain will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Synaptic Weight:** `DelaySynapse` now has a configurable `weight: f32` field.
  `transmit()` multiplies output by weight instead of using if/else for
  inhibitory. Default weights (1.0/-1.0) produce identical behavior to v1.0.0.
- **STDP (Spike-Timing Dependent Plasticity):** New `plasticity` module in core
  crate with `StdpConfig`, `compute_stdp_delta()`, `apply_stdp()`. Enable via
  `brain.enable_stdp(config)` for online weight adaptation.
- **Homeostatic Plasticity:** `HomeostasisConfig` + `apply_homeostasis()` for
  automatic threshold adjustment. Overactive neurons raise threshold, quiet
  neurons lower it. Enable via `brain.enable_homeostasis(config)`.
- **Neuron Activity Tracking:** `activity_ema` (exponential moving average) and
  `last_spike_step` fields added to `Neuron` for plasticity mechanisms.
- **37 new plasticity tests:** STDP direction/decay/symmetry/bounds, homeostasis
  up/down/stable/bounds, brain integration, determinism, combined operation.

## [1.0.0] — 2026-04-06

### Added
- **Core Engine:** Biomorphic 5-neuron Muenster circuit (AN1, LN2, LN3, LN5, ON1)
  with Gaussian resonators, delay-line synapses, and coincidence detection.
- **Sequence Predictor (v0.3):** N-gram pattern matching with confidence scoring
  (`C = SNR / (1+SNR) * (1 - jitter/tolerance)`), debounced token detection,
  and multi-pattern competition.
- **Resonator Bank (v0.2):** Parallel multi-token detection with one 5-neuron
  channel per token. Optional Rayon parallelism via `parallel` feature.
- **Token Vocabulary (v0.2):** Multi-frequency token encoding. Alphabet mode
  (27 tokens, 2000-8000 Hz) and custom label support.
- **Morse Code:** Full A-Z + 0-9 encode/decode with configurable timing.
- **FFI Bindings:** C-compatible API (`brain_new`, `brain_step`, `brain_free`,
  `brain_get_status`) with generated `cricket_brain.h` header.
- **Python Bindings:** PyO3 classes `BrainConfig` and `Brain` with `step()`,
  `step_batch()`, `reset()`, and privacy mode support.
- **WASM Bindings:** `wasm-bindgen` `Brain` class with telemetry drain and
  prediction snapshots for browser integration.
- **Telemetry System:** `Telemetry` trait with events: Spike, ResonanceChange,
  SequenceMatched, SnrReport, SystemOverload. JSON Lines sink for dashboards.
- **Privacy Mode:** Timestamp anonymization and value coarsening for HIPAA/GDPR
  compliance via `BrainConfig::privacy_mode`.
- **Adaptive Sensitivity (AGC):** Automatic gain control with exponential
  moving average on input energy.
- **Chaos Detection:** Shannon entropy monitoring with overload alerting when
  entropy > 3.2 and >80% neurons active.
- **Snapshot/Restore:** Full serializable state with CRC64 checksum and version
  hash verification (requires `serde` feature).
- **no_std Core:** `crates/core` runs without standard library (only `libm`),
  enforced with `#![deny(unsafe_code)]`.
- **Examples:** 14 reference implementations including ECG sentinel monitor,
  research SNR sweep generator, CLI with config/snapshot support, Arduino
  minimal (944 bytes RAM), and WASM browser demo.
- **CI Pipeline:** Cross-platform tests (Linux/macOS/Windows), rustfmt, clippy,
  WASM build verification, FFI header sync check, and security audit.
- **Documentation:** Project dossier, research whitepaper, commercial licensing
  guide, and security policy.

### Changed
- **License:** Switched from MIT/Apache-2.0 to AGPL-3.0 + Commercial dual license.
  Commercial use in proprietary software requires a paid license.

### Security
- Upgraded `pyo3` from 0.22 to 0.24 to patch RUSTSEC-2025-0020 (buffer overflow
  in `PyString::from_object`).

## [0.3.0] — 2026-04-05

### Added
- Sequence prediction engine with temporal pattern matching.
- Confidence scoring model with SNR and jitter metrics.
- Privacy mode for HIPAA/GDPR-compliant telemetry.
- Dual licensing (MIT OR Apache-2.0).

## [0.2.0] — 2026-04-04

### Added
- Multi-frequency token vocabulary and resonator bank.
- Live demo with encode-brain-decode roundtrip.
- Frequency discrimination example.

## [0.1.0] — 2026-04-03

### Added
- Initial implementation of Cricket-Brain neuromorphic engine.
- 5-neuron canonical circuit based on Muenster model.
- Basic Morse code encoding/decoding.
- Criterion benchmarks (0.175 us/step canonical circuit).
