# Silent Sentinel Research Whitepaper (Draft)

## Abstract

Silent Sentinel is a neuromorphic inference engine inspired by the cricket auditory pathway. Instead of gradient-based learning, it uses delay-line coincidence detection and resonant filters to detect temporal patterns with low latency and tiny memory footprints. This draft captures reproducible benchmark data, sweep tooling, and certification-grade evaluation outputs (SNR sweeps + ROC operating points).

## 1) Biological Inspiration: Resonator-Delay Architecture

The architecture models a compact insect auditory circuit:

- AN1-like receptor resonance tuned to carrier
- Local interneurons (inhibitory/excitatory) coupled by delayed synapses
- ON1-like output gate that fires on coincidence

This produces strong temporal selectivity while remaining computationally lightweight.

## 2) Mathematical Model

The system is governed by four primitives:

1. **Frequency selectivity** via Gaussian resonance:

\[
\text{match} = \exp\left(-\left(\frac{\Delta f}{f_0\,w}\right)^2\right)
\]

2. **Amplitude update** with bounded growth + decay.
3. **Phase locking** with gradual synchronization toward the input phase.
4. **Coincidence gate** requiring simultaneous present+delayed evidence.

For a complete derivation, see `docs/math.md`.

## 3) Performance Metrics (μs/step, memory, throughput)

### Run 8 Memory Baseline

| Configuration | Memory footprint | Notes |
|---|---:|---|
| no_std Arduino minimal | **944 bytes** | Static array implementation, no heap |
| 40,960-neuron scale brain | **13.91 MB** | Runtime measured (`scale_test`) |
| v0.3 predictor (1280 neurons) | **0.30 MB** | Runtime measured (`scale_predict`) |

### Run 14 Throughput Baseline (refresh: 2026-04-06)

| Example | Throughput / Latency | Notes |
|---|---:|---|
| `profile_speed` | **0.175431 μs/step** | Canonical 5-neuron circuit |
| `scale_test` | **3.43e7 neuron-ops/sec** | 40,960 neurons, single-thread CPU |
| `scale_predict` | **3.32e7 neuron-ops/sec** | Sequence predictor mode |

## 4) Sentinel Results: SNR Sweep + ROC Data

A new headless data-harvesting utility (`examples/research_gen.rs`) performs a parametric sweep across:

- **SNR**: -10 dB to +30 dB (5 dB increments)
- **Sensitivity grid**: 0.20 to 0.90
- **Output**:
  - `sentinel_sweep.csv` for plotting pipelines
  - `sentinel_sweep.json` for reproducibility + metadata

### Certification-oriented Metrics

For each (SNR, sensitivity) pair, the generator logs:

- TP, FP, TN, FN
- TPR (Detection Rate)
- FPR (False Positive Rate)

This supports direct construction of ROC curves expected in medical/security-style certification dossiers.

## 5) Reproducibility Protocol

To guarantee replicability:

1. Set deterministic seed in `BrainConfig::with_seed(...)`.
2. Run headless generator:

```bash
cargo run --release --example research_gen -- --headless --output target/research --seed 1337
```

3. Archive generated CSV/JSON with commit hash and platform metadata.

## 6) Next Steps for Submission

- Add confidence intervals over repeated sweeps (bootstrap).
- Add comparisons versus classical matched-filter and threshold baselines.
- Add figure pipeline (Python/R) that consumes `sentinel_sweep.csv` and emits publication-ready plots.
- Freeze benchmark environment (CPU model, rustc version, flags) for camera-ready appendix.
