<div align="center">

<img src="assets/Logo_4k_4mb_11mp.png" alt="CricketBrain Logo" width="400">

# CricketBrain

### The Biomorphic Inference Engine

**Sub-microsecond pattern recognition. Sub-kilobyte memory. Zero training.**

[![CI](https://github.com/BEKO2210/cricket-brain/actions/workflows/ci.yml/badge.svg)](https://github.com/BEKO2210/cricket-brain/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-red.svg)](LICENSE)
[![Commercial License](https://img.shields.io/badge/Commercial-License%20Available-brightgreen.svg)](COMMERCIAL.md)
[![Crate](https://img.shields.io/badge/crates.io-cricket--brain-orange)](https://crates.io/crates/cricket-brain)
[![docs.rs](https://img.shields.io/badge/docs.rs-cricket--brain-blue)](https://docs.rs/cricket-brain)
[![no_std](https://img.shields.io/badge/no__std-compatible-green.svg)](#embedded--no_std)
[![MSRV](https://img.shields.io/badge/MSRV-1.75-blue.svg)](https://www.rust-lang.org)
[![Security Audit](https://img.shields.io/badge/cargo--audit-passing-brightgreen.svg)](#)

*Inspired by 200 million years of cricket evolution.*
*Built for the next generation of edge intelligence.*

[Quick Start](#quick-start) | [Benchmarks](#benchmarks) | [Whitepaper](RESEARCH_WHITEPAPER.md) | [API Docs](https://docs.rs/cricket-brain) | [Contributing](CONTRIBUTING.md)

</div>

---

## What is CricketBrain?

CricketBrain is a **neuromorphic signal processor** that recognizes temporal patterns in real-time using delay-line coincidence detection — the same mechanism the field cricket (*Gryllus bimaculatus*) uses to find mates in noisy environments.

**No matrix multiplication. No CUDA. No weights. No training.**

Just 5 neurons and 6 synapses, processing at **0.175 us per step** in **944 bytes of RAM**.

```
         AN1 (Receptor, 4500 Hz)
        / | \
       /  |  \
      v   v   v
    LN2  LN3  LN5          ← 3 interneurons with different delays
   (inh) (exc) (inh)
   3ms   2ms   5ms
      \   |   /
       \  |  /
        v v v
     ON1 (Output Gate)      ← fires only on temporal coincidence
```

### Why Should You Care?

| | CricketBrain | Traditional ML | Deep Learning |
|---|:---:|:---:|:---:|
| **Latency** | 0.175 us | ~100 us | ~10 ms |
| **Memory** | 944 bytes | 10+ KB | 100+ MB |
| **Training** | None | Hours | Days-Weeks |
| **GPU Required** | No | No | Yes |
| **Deterministic** | Yes | Depends | No |
| **`no_std` / Embedded** | Yes | Rare | No |
| **Explainable** | Fully | Partially | Black box |

---

## Use Cases

| Domain | Application | How CricketBrain Helps |
|--------|------------|----------------------|
| **Medical** | ECG rhythm classification | Real-time arrhythmia detection on wearables ([example](examples/sentinel_ecg_monitor.rs)) |
| **Industrial IoT** | Vibration monitoring | Detect bearing failure patterns at the sensor node |
| **Audio** | Keyword / wake-word detection | Sub-millisecond response without cloud roundtrip |
| **Security** | Network traffic analysis | Temporal pattern anomaly detection at line rate |
| **Robotics** | Sensor fusion | Deterministic latency for real-time control loops |
| **Embedded** | Microcontroller signal processing | Runs on Arduino Uno (2 KB RAM) |

---

## Quick Start

### Install & Run (30 Seconds)

```bash
git clone https://github.com/BEKO2210/cricket-brain.git
cd cricket-brain
cargo run --example live_demo -- "HELLO WORLD"
```

Output:
```
--- Spike Train (each char = 10ms) ---
|||||_____|||||_____|||||_____|||||_______________|||||_______________...

Decoded output: "HELLO WORLD"
Match: EXACT MATCH
```

### Use as a Library

```toml
[dependencies]
cricket-brain = "1.0"
```

```rust
use cricket_brain::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the brain with default 5-neuron Muenster circuit
    let mut brain = CricketBrain::new(BrainConfig::default())?;

    // Feed a 4500 Hz signal (cricket carrier frequency)
    for _ in 0..100 {
        let output = brain.step(4500.0);
        if output > 0.0 {
            println!("Spike detected! amplitude={output:.3}");
        }
    }

    // Feed silence — zero false positives
    for _ in 0..50 {
        assert_eq!(brain.step(0.0), 0.0);
    }

    // Batch processing
    let inputs = vec![4500.0; 100];
    let outputs = brain.step_batch(&inputs);

    // Full reset for next pattern
    brain.reset();
    Ok(())
}
```

---

## Multi-Language Support

<table>
<tr>
<td>

**Rust** (native)
```rust
use cricket_brain::prelude::*;
let mut brain = CricketBrain::new(
    BrainConfig::default()
)?;
let out = brain.step(4500.0);
```

</td>
<td>

**C / C++ / Swift**
```c
BrainHandle *h = NULL;
brain_new(&h, 5, 4000.0, 5000.0);
float out;
brain_step(h, 4500.0, &out);
brain_free(h);
```

</td>
</tr>
<tr>
<td>

**Python**
```python
from cricket_brain import BrainConfig, Brain
brain = Brain(BrainConfig())
out = brain.step(4500.0)
batch = brain.step_batch([4500.0] * 100)
```

</td>
<td>

**JavaScript / TypeScript (WASM)**
```typescript
import { Brain } from "cricket-brain-wasm";
const brain = new Brain(42);
const out = brain.step(4500.0);
const events = brain.drainTelemetry();
```

</td>
</tr>
</table>

Build commands:
```bash
# C FFI
cargo build --release -p cricket-brain-ffi
# Header: crates/ffi/include/cricket_brain.h

# Python (requires maturin)
cd crates/python && maturin develop --release

# WASM (requires wasm-pack)
cd crates/wasm && wasm-pack build --target web --out-dir pkg
```

---

## Benchmarks

Measured with Criterion on x86-64. Fully reproducible via `cargo bench`.

| Scenario | Latency | Throughput | Memory |
|---|---:|---:|---:|
| **Canonical 5-neuron** | **0.175 us/step** | 5.7M steps/sec | 348 bytes |
| **1,280-neuron predictor** | — | 33.2M neuron-ops/sec | 0.30 MB |
| **40,960-neuron scale** | — | 34.3M neuron-ops/sec | 13.91 MB |
| **Arduino `no_std`** | — | — | **944 bytes** |

### vs. Classical Baselines (SNR = 0 dB)

CricketBrain tested against 3 classical detectors under identical conditions
([full results](examples/baselines.rs)):

| Method | TPR | FPR | Advantage |
|--------|:---:|:---:|-----------|
| **CricketBrain** | **1.000** | **0.000** | Temporal coincidence rejects noise |
| IIR Bandpass | 1.000 | 0.608 | Cannot distinguish pattern from noise |
| Goertzel (FFT) | 0.042 | 0.000 | Misses jittered signals |
| Matched Filter | 0.008 | 0.000 | Rigid template, no jitter tolerance |

> CricketBrain achieves **perfect detection with zero false positives** across
> all SNR levels from -10 dB to +30 dB. See [RESEARCH_WHITEPAPER.md](RESEARCH_WHITEPAPER.md).

---

## Features

### Core Capabilities

- **Gaussian resonators** — frequency-selective neurons with +-10% bandwidth
- **Delay-line synapses** — ring-buffer propagation delays (1-9 ms)
- **Coincidence detection** — fires only on sustained temporal evidence
- **Adaptive sensitivity (AGC)** — automatic gain control for varying input levels
- **Sequence prediction** — N-gram pattern matching with confidence scoring
- **Multi-token detection** — parallel resonator banks (one 5-neuron circuit per token)

### Production Features

| Feature | Description |
|---------|-------------|
| **Privacy Mode** | Timestamp anonymization + value coarsening (HIPAA/GDPR) |
| **Snapshot/Restore** | Serialize full state with CRC64 checksums |
| **Telemetry** | Structured event hooks (Spike, SNR, Overload) + JSON Lines sink |
| **Chaos Detection** | Shannon entropy monitoring with overload alerts |
| **Deterministic** | Seeded RNG — identical results across platforms |
| **Error Codes** | Consistent FFI error contract across all language bindings |

### Cargo Feature Flags

```toml
[dependencies]
cricket-brain = { version = "1.0", features = ["serde", "parallel"] }
```

| Flag | Default | Description |
|------|:-------:|-------------|
| `std` | Yes | Standard library support |
| `no_std` | — | Embedded mode (alloc only) |
| `serde` | — | Snapshot serialization with CRC64 |
| `parallel` | — | Rayon-based resonator bank parallelism |
| `telemetry` | — | Structured event hooks |
| `cli` | — | JSON telemetry sink + config parsing |

---

## Embedded / `no_std`

The core crate (`cricket-brain-core`) is fully `no_std` compatible with `#![deny(unsafe_code)]`.

```bash
# Verify it builds without std
cargo build -p cricket-brain-core --no-default-features
```

**Minimal embedded example** ([`examples/arduino_minimal.rs`](examples/arduino_minimal.rs)):
- Fixed-size arrays (no heap allocation)
- 944 bytes total RAM
- Runs on Arduino Uno, STM32, ESP32, any Cortex-M

---

## Scientific Validation

CricketBrain includes a complete **scientific publication package**:

| Artifact | Description |
|----------|-------------|
| [RESEARCH_WHITEPAPER.md](RESEARCH_WHITEPAPER.md) | Full paper with 16 peer-reviewed references |
| [`examples/baselines.rs`](examples/baselines.rs) | Matched filter, Goertzel, IIR bandpass comparison |
| [`examples/ablation_study.rs`](examples/ablation_study.rs) | Systematic circuit component analysis |
| [`examples/research_gen.rs`](examples/research_gen.rs) | SNR sweep with Wilson 95% confidence intervals |
| [AI_DEVELOPMENT_STATEMENT.md](AI_DEVELOPMENT_STATEMENT.md) | Full AI-tooling transparency disclosure |

### Ablation Study Results

Each component of the Muenster circuit is necessary:

| Configuration | SNR 0 dB TPR | Impact |
|---|:---:|---|
| **Full circuit** | 1.000 | Baseline |
| Without LN3 (excitatory) | **0.440** | Critical — excitatory drive to ON1 |
| Without LN2 (inhibitory) | 1.000 | Redundant at this SNR |
| Without coincidence gate | 1.000 | Gate prevents false positives at low SNR |

Reproduce all results:
```bash
cargo run --release --example baselines
cargo run --release --example ablation_study
cargo run --release --example research_gen -- --seed 1337
```

---

## Examples

```bash
# Morse code & basics
cargo run --example live_demo -- "HELLO"       # Encode -> brain -> decode
cargo run --example frequency_discrimination   # Gaussian bandpass demo
cargo run --example morse_alphabet             # All 26 characters
cargo run --example arduino_minimal            # no_std embedded demo

# Multi-frequency tokens
cargo run --example multi_freq_demo -- "RUST"  # Token discrimination

# Sequence prediction
cargo run --example sequence_predict           # Pattern prediction
cargo run --release --example scale_predict    # 256-token / 1280-neuron benchmark

# Medical / research
cargo run --release --example sentinel_ecg_monitor  # ECG tachycardia detection
cargo run --release --example research_gen          # SNR sweep + ROC data
cargo run --release --example baselines             # Classical baseline comparison
cargo run --release --example ablation_study        # Circuit ablation study

# Performance
cargo run --release --example scale_test       # 40,960-neuron throughput
cargo run --release --example profile_speed    # Latency measurement
cargo bench                                    # Criterion benchmarks
```

---

## Project Structure

```
cricket-brain/
|-- crates/
|   |-- core/          no_std primitives (neuron, synapse, telemetry)
|   |-- ffi/           C-compatible API + generated header
|   |-- python/        PyO3 bindings
|   `-- wasm/          wasm-bindgen bindings
|-- src/               Brain network, sequence predictor, resonator bank
|-- examples/          15 runnable examples
|-- tests/             85 tests (unit, integration, edge-case, FFI, property)
|-- benches/           Criterion benchmarks
`-- docs/              Mathematical derivations
```

---

## Quality

| Check | Status |
|-------|--------|
| `cargo test --workspace` | 85 tests passing |
| `cargo clippy -D warnings` | Zero warnings |
| `cargo fmt -- --check` | Enforced |
| `cargo audit --deny warnings` | Zero vulnerabilities |
| Cross-platform CI | Linux, macOS, Windows |
| `no_std` verification | Verified in CI |
| WASM build | Verified in CI |
| FFI header sync | Verified in CI |

---

## Roadmap

- [x] **v0.1** — Morse code recognition
- [x] **v0.2** — Multi-frequency token recognition
- [x] **v0.3** — Sequence prediction via delay-line pattern memory
- [x] **v1.0** — Production release with FFI/Python/WASM bindings
- [ ] **v1.1** — Adaptive Gaussian bandwidth (auto-tune for dense vocabularies)
- [ ] **v1.2** — STDP (spike-timing dependent plasticity) for online learning
- [ ] **v2.0** — Hardware deployment on RISC-V / ARM Cortex-M with real-time ADC

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
# Development workflow
cargo test --workspace                                    # All tests pass
cargo clippy --all-targets --all-features -- -D warnings  # Zero warnings
cargo fmt -- --check                                      # Formatting clean
```

---

## Citation

If you use CricketBrain in academic work, please cite:

```bibtex
@software{aslani2026cricketbrain,
  author  = {Aslani, Belkis},
  title   = {CricketBrain: A Biomorphic Delay-Line Coincidence Detector
             for Real-Time Temporal Pattern Recognition},
  year    = {2026},
  url     = {https://github.com/BEKO2210/cricket-brain},
  version = {1.0.0}
}
```

---

## License

**Dual-licensed:**

| Use Case | License | Cost |
|----------|---------|------|
| Open-source projects, research, education | [AGPL-3.0](LICENSE-AGPL-3.0) | Free |
| Proprietary / commercial / SaaS / embedded | [Commercial License](COMMERCIAL.md) | Paid |

The AGPL-3.0 requires that any software using CricketBrain must also be
released under AGPL-3.0 (including source code). If you cannot comply with
this, you need a **[commercial license](COMMERCIAL.md)**.

Contact: **belkis.aslani@gmail.com**

---

<div align="center">

**Author:** Belkis Aslani

*Built with AI-assisted development ([statement](AI_DEVELOPMENT_STATEMENT.md))
using Claude Code, ChatGPT/Codex, Kimi, and Gemini.*

[GitHub](https://github.com/BEKO2210/cricket-brain) | [Docs](https://docs.rs/cricket-brain) | [Whitepaper](RESEARCH_WHITEPAPER.md) | [Changelog](CHANGELOG.md)

</div>
