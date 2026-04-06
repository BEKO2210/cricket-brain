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
| RAM | ~848 bytes | ~800 GB |
| FLOPS/step | ~30 FP ops | ~3.1×10¹⁷ |
| GPU required | No | Yes (8× A100) |
| Training | None (hardwired) | Months on cluster |
| `no_std` | Yes | No |

## Bringing the AI to Life

### Step 1: Install & Run (30 seconds)

```bash
git clone https://github.com/BEKO2210/cricket-brain.git
cd cricket-brain
cargo run
```

This runs the SOS Morse demo. You'll see the brain detect tone segments and produce zero false positives during silence.

### Step 2: Encode any text and watch it think

```bash
# Full roundtrip: text → Morse → brain → spikes → decoded text
cargo run --example live_demo -- "HELLO WORLD"
```

Output:
```
--- Spike Train (each char = 10ms) ---
|||||_____|||||_____|||||_____|||||_______________|||||_______________...

Decoded output: "HELLO WORLD"
Match: EXACT MATCH
```

The `|` characters are spike events, `_` is silence. You can literally **see** the brain thinking — dots are short bursts, dashes are long bursts, gaps separate characters.

### Step 3: See frequency discrimination

```bash
cargo run --example frequency_discrimination
```

The brain is tuned to 4500 Hz. Frequencies outside the ±10% Gaussian window produce **zero** spikes:

```
    3500 Hz        0      200      0.0%
    4500 Hz      191      200     95.5%  |||||||||||||||||||||
    5500 Hz        0      200      0.0%
```

### Step 4: Scale it up

```bash
cargo run --release --example scale_test
```

Creates a 40,960-neuron brain, measures throughput:
- Init: ~13ms
- Memory: ~14 MB
- Throughput: ~40M neuron-ops/sec (single-threaded, no GPU)

### Step 5: Run all 26 Morse characters

```bash
cargo run --example morse_alphabet
```

Each letter produces a unique spike rate fingerprint.

### Step 6: Use it as a library in your own project

```toml
# In your Cargo.toml
[dependencies]
cricket-brain = { path = "../cricket-brain" }
```

```rust
use cricket_brain::brain::CricketBrain;
use cricket_brain::patterns::encode_morse;

fn main() {
    let mut brain = CricketBrain::new();

    // Option A: Feed a Morse-encoded message
    let signal = encode_morse("SOS");
    for &(freq, duration) in &signal {
        for _ in 0..duration {
            let spike = brain.step(freq);
            if spike > 0.0 {
                println!("Spike! {spike:.3}");
            }
        }
    }

    // Option B: Feed raw frequency data
    let output = brain.step(4500.0);  // cricket carrier
    let output = brain.step(0.0);     // silence

    // Option C: Batch processing
    let inputs = vec![4500.0; 100];
    let outputs = brain.step_batch(&inputs);

    // Reset for next pattern
    brain.reset();
}
```

### Step 7: Embed on a microcontroller

```bash
cargo run --example arduino_minimal
```

The `no_std` example uses fixed-size arrays (no heap) and fits in **944 bytes** — well within an Arduino Uno's 2 KB RAM. See [examples/arduino_minimal.rs](examples/arduino_minimal.rs) for the complete implementation.

## Quick Reference

```bash
cargo run                                    # SOS demo
cargo run --example live_demo -- "TEXT"       # Full encode→brain→decode roundtrip
cargo run --example frequency_discrimination # Bandpass filter demo
cargo run --example morse_alphabet           # All 26 letters
cargo run --example arduino_minimal          # no_std microcontroller demo
cargo run --release --example scale_test     # 40k neuron benchmark
cargo test                                   # Run all 31 tests
cargo bench                                  # Criterion throughput benchmarks
```

## Why Cricket Neuroscience?

Female crickets can locate a singing male in complete darkness, using a neural circuit of just 5 interneurons. This circuit doesn't learn — it's hardwired by evolution to detect a specific temporal pattern (the pulse interval of the species' song).

The key mechanism is **delay-line coincidence detection**: signals travel through synapses with different delays, and the output neuron only fires when signals from multiple paths arrive simultaneously. This is equivalent to a matched filter in signal processing, but implemented with biological hardware that fits in a few hundred bytes of memory.

Cricket-Brain takes this principle and applies it to arbitrary pattern recognition tasks like Morse code, rhythm detection, and temporal sequence classification.

## What Can It Actually Do?

**What works today (v0.1):**
- Frequency-selective signal detection (Gaussian bandpass, ±10% bandwidth)
- Temporal pattern encoding/decoding (Morse code A-Z, 0-9)
- Full roundtrip: text → Morse → neuromorphic processing → spike train → text
- Zero false positives during silence (coincidence gate prevents noise)
- Scales to 40,960+ neurons on a single CPU thread
- Runs on Arduino Uno (944 bytes RAM, no heap)

**What it is NOT (yet):**
- Not a general-purpose language model
- Not a classifier that learns from data (topology is hand-designed)
- Not a replacement for neural networks on high-dimensional tasks
- The Morse decoder uses spike timing analysis, not pure neural output

**The real value proposition:** This is a physically-valid computational model that processes temporal patterns with near-zero resources. It proves that useful inference is possible without gradient descent, backpropagation, or GPU clusters.

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
│   ├── live_demo.rs       Full encode→brain→decode roundtrip
│   ├── frequency_discrimination.rs  Bandpass filter visualization
│   ├── morse_alphabet.rs  All 26 letters
│   ├── arduino_minimal.rs no_std with fixed arrays (944 bytes)
│   └── scale_test.rs     40,960 neuron benchmark
├── tests/                 Integration tests (31 tests)
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
