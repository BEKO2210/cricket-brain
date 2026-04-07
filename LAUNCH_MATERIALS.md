# CricketBrain v1.1 — Launch Materials

> **Core positioning:** CricketBrain combines hardwired delay-line coincidence
> detection with optional adaptive plasticity: weighted synapses, STDP learning,
> and homeostatic threshold regulation — in ~1 KB RAM at 97 ns/step.

---

## 1. Dev.to / Blog Post

### Title: "I built an adaptive neuromorphic engine in Rust: STDP learning in 1 KB of RAM at 97 nanoseconds per step"

**Tags:** rust, opensource, embedded, machinelearning, neuroscience

---

What if an insect brain could teach us how to build better edge AI?

The field cricket (*Gryllus bimaculatus*) recognizes songs using 5 neurons
and delay-line coincidence detection. I took that biological circuit, built
it in Rust, and then added something the cricket doesn't have: **adaptive
learning**.

**CricketBrain v1.1** is a neuromorphic signal processor that combines a
hardwired core with optional STDP (Spike-Timing Dependent Plasticity) and
homeostatic threshold regulation. It processes signals at **97 nanoseconds
per step** in about **1 KB of RAM**.

### The architecture

The core is a 5-neuron Muenster circuit — the same topology neuroscientists
mapped in real crickets (Schoeneich et al., 2015):

1. **AN1** — receptor neuron, Gaussian frequency selectivity
2. **LN2, LN3, LN5** — interneurons with different axonal delays (2-5 ms)
3. **ON1** — coincidence gate, fires only on correct temporal pattern

What's new in v1.1: every synapse now carries a **weight** that can adapt
online via STDP. Neurons track their activity and adjust firing thresholds
via homeostasis. The network can learn from temporal patterns — not just
detect hardcoded ones.

### Adaptive plasticity

```rust
use cricket_brain::prelude::*;
use cricket_brain::plasticity::{StdpConfig, HomeostasisConfig};

let mut brain = CricketBrain::new(BrainConfig::default())?;

// Enable online adaptation
brain.enable_stdp(StdpConfig::default().with_learning_rate(0.02));
brain.enable_homeostasis(HomeostasisConfig::default());

// Feed signal — weights and thresholds adapt in real-time
for &freq in &signal {
    brain.step(freq);
}

// Inspect what the network learned
for syn in &brain.synapses {
    println!("{}->{}: weight={:.3}", syn.from, syn.to, syn.weight);
}
```

### Key numbers

| Metric | Value |
|--------|-------|
| Latency | 97 ns/step (Criterion-measured, x86-64) |
| Memory | ~1008 bytes (no_std, calculated) |
| Detection | TPR 1.0, FPR 0.0 (synthetic benchmarks) |
| Dependencies | 1 (libm) in core crate |
| Learning | STDP + homeostatic thresholds (opt-in) |
| Platforms | Linux, macOS, Windows, WASM (CI-verified) |
| Tests | 122 (including 37 plasticity-specific) |

### vs. classical detectors (SNR = 0 dB)

| Method | TPR | FPR |
|--------|:---:|:---:|
| **CricketBrain** | **1.000** | **0.000** |
| IIR Bandpass | 1.000 | 0.558 |
| Matched Filter | 0.000 | 0.000 |

**Caveat:** These are synthetic benchmarks. Real-world performance depends
on the application.

### What it is and what it isn't

**What it is:**
- An ultralight adaptive neuromorphic signal processor
- A `no_std` Rust library with C, Python, and WASM bindings
- A research tool grounded in published neuroscience (16 references)

**What it isn't:**
- A general-purpose ML framework
- A validated medical device
- Tested on real embedded hardware (designed for it, verified on host)

### Try it

```bash
git clone https://github.com/BEKO2210/cricket-brain.git
cd cricket-brain
cargo run --example live_demo -- "HELLO WORLD"
cargo run --example sequence_predict
cargo bench
```

GitHub: https://github.com/BEKO2210/cricket-brain

I'd love feedback — especially from embedded engineers, neuroscience
researchers, and anyone working on edge computing.

---

## 2. Reddit r/rust

### Title: "CricketBrain v1.1 — adaptive neuromorphic engine: STDP learning + homeostasis in no_std Rust (~1 KB, 97 ns/step)"

CricketBrain v1.1 adds online plasticity to a biomorphic signal processor
modeled after cricket hearing.

**What's new in v1.1:**
- Synaptic weights (configurable, default +-1.0)
- STDP learning: weights adapt based on pre/post spike timing
- Homeostatic thresholds: neurons auto-regulate activity level
- 37 new plasticity tests (122 total)
- Still ~1 KB, still 97 ns/step, still `no_std`

**Architecture:** 5-neuron Muenster circuit with delay-line coincidence
detection. The hardwired topology provides the base detection, STDP allows
online adaptation to patterns.

**What I'd appreciate feedback on:**
- Is the STDP implementation reasonable for embedded targets?
- `no_std` with 1 dep (libm) — viable for Cortex-M?
- The adaptive bandwidth mechanism for dense token vocabularies

```rust
brain.enable_stdp(StdpConfig::default().with_learning_rate(0.02));
brain.enable_homeostasis(HomeostasisConfig::default());
```

Repo: https://github.com/BEKO2210/cricket-brain

Built with AI-assisted development — full transparency statement in repo.

---

## 3. Reddit r/embedded

### Title: "no_std adaptive neuromorphic engine: STDP + homeostasis in ~1 KB RAM, 1 dep (libm) — feedback wanted for Cortex-M"

CricketBrain v1.1 adds synaptic plasticity to a biomorphic signal processor.
Core is `no_std` with `#![deny(unsafe_code)]` and a single dep (libm).

**Memory budget:**
- 5-neuron circuit: ~1008 bytes (calculated from struct sizes)
- Includes: 5 neurons with history buffers, 6 synapses with delay lines,
  synaptic weights, activity EMA, spike timestamps

**New adaptive features (all opt-in):**
- STDP: `Δw = η·exp(-|Δt|/τ)` — online weight adaptation
- Homeostasis: threshold adjusts to maintain target activity
- Weights clamped to [-2.0, 2.0], thresholds to [0.3, 0.95]

Honest question: has anyone deployed a similar Rust `no_std` adaptive
system on real hardware? I've only verified on host so far.

https://github.com/BEKO2210/cricket-brain

---

## 4. Hacker News

### Title: CricketBrain v1.1: Adaptive neuromorphic engine in Rust — STDP, homeostasis, ~1 KB, 97 ns/step

### URL: https://github.com/BEKO2210/cricket-brain

---

## 5. Twitter/X Thread

**Tweet 1:**
CricketBrain v1.1 is out — an adaptive neuromorphic signal processor in Rust.

Hardwired delay-line core + optional STDP learning + homeostatic thresholds.

97 ns/step. ~1 KB RAM. no_std. 1 dependency.

The network can now learn from temporal patterns, not just detect them.

https://github.com/BEKO2210/cricket-brain

🧵 (1/5)

**Tweet 2:**
What's new in v1.1:

- Synaptic weights: every connection is now weighted
- STDP: pre-before-post = strengthen, post-before-pre = weaken
- Homeostasis: overactive neurons raise threshold, quiet ones lower it
- 37 new tests (122 total)

All opt-in. Default behavior = identical to v1.0.

(2/5)

**Tweet 3:**
The science: STDP is the same learning rule found in biological neurons.

Δw = η · exp(-|Δt| / τ)

In CricketBrain, it runs in constant time per synapse, no alloc, no_std.

Combined with the delay-line coincidence gate, the network adapts its
temporal selectivity online.

(3/5)

**Tweet 4:**
Performance after adding plasticity:

Before: 85 ns/step, 944 bytes
After:  97 ns/step, ~1008 bytes

+14% latency for fully adaptive learning. Still under 100 ns.
Still under 1.1 KB. Still no_std.

(4/5)

**Tweet 5:**
Looking for:
- Embedded engineers to try this on Cortex-M
- Neuroscience researchers for real signal datasets
- Edge computing folks for use-case validation

AGPL-3.0 open source. Commercial license available.

What temporal patterns would you teach it?

(5/5)

---

## 6. LinkedIn

**CricketBrain v1.1 — Adaptive Neuromorphic Signal Processing in Rust**

CricketBrain v1.1 adds online adaptive learning to the neuromorphic core:

- **STDP (Spike-Timing Dependent Plasticity):** Synaptic weights adapt
  based on the relative timing of neural activity — the same learning
  rule found in biological brains.
- **Homeostatic regulation:** Neuron firing thresholds automatically adjust
  to maintain stable network activity.
- **Weighted synapses:** Every connection now carries a configurable weight
  that plasticity rules can modify online.

The core architecture remains a hardwired 5-neuron circuit inspired by
cricket hearing (Schoeneich et al., 2015), but it can now adapt to
temporal patterns instead of relying solely on fixed registration.

Technical highlights:
- 97 ns per processing step (Criterion-measured)
- ~1 KB total RAM with all plasticity features
- 122 tests including 37 plasticity-specific
- Rust, no_std core, single dependency (libm)
- C/C++, Python, and WebAssembly bindings
- AGPL-3.0 open source + commercial licensing

The project includes a research whitepaper with 16 peer-reviewed references,
baseline comparisons, an ablation study, and full AI-development transparency.

GitHub: https://github.com/BEKO2210/cricket-brain

#Rust #NeuromorphicComputing #STDP #EdgeAI #OpenSource #Embedded

---

## 7. Rust Weekly Newsletter

**Email to:** thisweekinrust@gmail.com

**Subject:** CricketBrain v1.1 — adaptive neuromorphic engine (STDP, no_std, 1 dep)

Hi,

CricketBrain v1.1 adds adaptive plasticity to a neuromorphic signal
processor inspired by cricket auditory circuits:

- **New:** STDP learning, weighted synapses, homeostatic thresholds
- **Core:** no_std, 1 dep (libm), #![deny(unsafe_code)], 97 ns/step
- **Bindings:** C FFI, Python (PyO3), WASM
- **Tests:** 122 (37 plasticity-specific)
- **License:** AGPL-3.0 + Commercial

Category: Crate of the Week / Interesting Projects
URL: https://github.com/BEKO2210/cricket-brain

Thank you,
Belkis Aslani
