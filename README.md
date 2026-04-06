# Cricket-Brain

**A biomorphic AI inference engine based on the Münster model of cricket hearing.**

![no_std](https://img.shields.io/badge/no__std-compatible-green)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow)

Cricket-Brain uses **delay-line coincidence detection** for pattern recognition — no matrix multiplication, no CUDA, no weights. The architecture is modeled after the auditory processing circuit of the field cricket (*Gryllus bimaculatus*), where just 5 neurons can recognize species-specific song patterns in real time.

## Architecture

```
Sound (4500 Hz pulse train)
        │
        ▼
       AN1 ─── Gaussian resonator (freq-selective)
        │
        ├──▶ LN2 (inh, -3ms) ──▶ ON1
        │
        ├──▶ LN5 (inh, -5ms) ──▶ ON1
        │
        └──▶ LN3 (exc, +2ms) ──▶ ON1
                                   │
                                   ▼
                            Coincidence Gate
                            (fire if now ∧ delayed)
```

| Property | Cricket-Brain | GPT-4 |
|----------|--------------|-------|
| Parameters | 5 neurons, 6 synapses | ~1.8T parameters |
| RAM | ~600 bytes | ~800 GB |
| FLOPS/step | ~30 FP ops | ~3.1×10¹⁷ |
| GPU required | No | Yes (8× A100) |
| Training | None (hardwired) | Months on cluster |
| `no_std` | Yes | No |

## Quick Start

```bash
# Run the SOS Morse demo
cargo run

# Run the full alphabet demo
cargo run --example morse_alphabet

# Run the scale test (40,960 neurons)
cargo run --example scale_test

# Run benchmarks
cargo bench

# Run tests
cargo test
```

## Why Cricket Neuroscience?

Female crickets can locate a singing male in complete darkness, using a neural circuit of just 5 interneurons. This circuit doesn't learn — it's hardwired by evolution to detect a specific temporal pattern (the pulse interval of the species' song).

The key mechanism is **delay-line coincidence detection**: signals travel through synapses with different delays, and the output neuron only fires when signals from multiple paths arrive simultaneously. This is equivalent to a matched filter in signal processing, but implemented with biological hardware that fits in a few hundred bytes of memory.

Cricket-Brain takes this principle and applies it to arbitrary pattern recognition tasks like Morse code, rhythm detection, and temporal sequence classification.

## Mathematical Foundation

### Gaussian Frequency Tuning

```
match = exp( -(Δf / f₀ / w)² )
```

Each neuron responds selectively to frequencies near its eigenfrequency `f₀`, with bandwidth parameter `w = 0.1` (10%).

### Amplitude Dynamics

```
If resonating:   A(t+1) = min( A(t) + match · 0.3,  1.0 )
If not:          A(t+1) = A(t) · 0.95
```

### Phase Locking

```
If resonating:   φ(t+1) = φ(t) + (φ_in - φ(t)) · 0.1
If not:          φ(t+1) = φ(t) · 0.98
```

### Coincidence Detection

```
fire = (A(t) > θ) ∧ (A(t - τ) > θ · 0.8)
```

The neuron fires only when both current and delayed amplitude exceed threshold — ensuring only sustained, correctly-timed patterns trigger output.

See [docs/math.md](docs/math.md) for the complete mathematical derivation.

## Project Structure

```
cricket-brain/
├── src/
│   ├── lib.rs           Public API
│   ├── main.rs          SOS Morse demo
│   ├── neuron.rs        Neuron + resonate() + coincidence detection
│   ├── synapse.rs       DelaySynapse + transmit()
│   ├── brain.rs         CricketBrain network + step()
│   └── patterns.rs      Morse code encoding/decoding
├── examples/
│   ├── morse_alphabet.rs  All 26 letters
│   ├── arduino_minimal.rs no_std with fixed arrays
│   └── scale_test.rs     40,960 neuron benchmark
├── tests/                 Integration tests
├── benches/               Criterion benchmarks
└── docs/                  Architecture & math docs
```

## Roadmap

- **v0.1** — Morse code recognition (current)
- **v0.2** — Multi-frequency token recognition (parallel resonator banks)
- **v0.3** — 40k resonator LLM-alternative for sequence prediction
- **v1.0** — Arduino port with real-time audio input via ADC

## License

MIT License — see [LICENSE](LICENSE) for details.

**Repository**: [github.com/BEKO2210/cricket-brain](https://github.com/BEKO2210/cricket-brain)
