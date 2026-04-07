# CricketBrain — Launch Materials

## 1. Dev.to / Blog Post

### Title: "I built a neuromorphic inference engine in Rust that processes signals in 0.175 microseconds with 944 bytes of RAM"

**Tags:** rust, opensource, embedded, machinelearning

---

What if I told you that a cricket — the insect — solves pattern recognition
problems faster and more efficiently than most machine learning models?

The field cricket (*Gryllus bimaculatus*) recognizes species-specific songs
using just 5 neurons and delay-line coincidence detection. No training.
No weights. No gradient descent. I took that biological circuit and built
it in Rust.

**The result: CricketBrain** — a neuromorphic signal processor that detects
temporal patterns in 0.175 microseconds per step, using 944 bytes of RAM.

### What it does

CricketBrain processes streaming signals (frequencies, sensor data, timing
patterns) and detects specific temporal structures. It works like a biological
auditory system:

1. **Gaussian resonators** filter for target frequencies
2. **Delay-line synapses** create timing-dependent pathways
3. **A coincidence gate** fires only when the timing pattern matches

```
        AN1 (Receptor)
       / | \
      v  v  v
    LN2 LN3 LN5    ← different delays: 3ms, 2ms, 5ms
      \  |  /
       v v v
      ON1 (Gate)    ← fires only on correct timing
```

### Real numbers

| Metric | Value |
|--------|-------|
| Latency | 0.175 us/step (Criterion-measured) |
| Memory | 944 bytes (calculated, no_std) |
| Detection | TPR 1.0, FPR 0.0 on synthetic benchmarks |
| Dependencies | 1 (libm) in core crate |
| Training | None — topology is hardwired |
| Platforms | Linux, macOS, Windows, WASM (CI-verified) |

### vs. Classical signal detection (SNR = 0 dB)

I compared CricketBrain against three classical detectors — matched filter,
Goertzel (FFT), and IIR bandpass — under identical conditions:

| Method | True Positive Rate | False Positive Rate |
|--------|:---:|:---:|
| **CricketBrain** | **1.000** | **0.000** |
| IIR Bandpass | 1.000 | 0.558 |
| Matched Filter | 0.000 | 0.000 |

The coincidence gate gives CricketBrain an inherent advantage: it rejects
noise that would trigger frequency-only detectors.

**Important caveat:** These results are on synthetic data with a controlled
signal generator. Real-world performance will depend on the specific application.

### How to try it

```bash
git clone https://github.com/BEKO2210/cricket-brain.git
cd cricket-brain
cargo run --example live_demo -- "HELLO WORLD"
```

Or use as a library:

```toml
[dependencies]
cricket-brain = "1.0"
```

```rust
use cricket_brain::prelude::*;

let mut brain = CricketBrain::new(BrainConfig::default())?;
let output = brain.step(4500.0); // feed a signal
```

### Bindings

CricketBrain ships with bindings for C/C++, Python, and WebAssembly:

```python
from cricket_brain import Brain
brain = Brain()
output = brain.step(4500.0)
```

### What it's NOT

- Not a replacement for neural networks on high-dimensional tasks
- Not a validated medical device
- Not tested on real embedded hardware yet (designed for it, verified on host)
- Patterns are registered, not learned — this is topology, not training

### The science

The architecture is based on published neuroscience research, particularly
Schoeneich, Kostarakos & Hedwig (2015) who mapped the AN1-LN2/LN3/LN5-ON1
circuit in *Gryllus bimaculatus*. The project includes a
[research whitepaper](https://github.com/BEKO2210/cricket-brain/blob/main/RESEARCH_WHITEPAPER.md)
with 16 peer-reviewed references, baseline comparisons, and an ablation study.

### Open source, dual licensed

CricketBrain is AGPL-3.0 for open source use. Commercial licenses are
available for proprietary applications.

GitHub: https://github.com/BEKO2210/cricket-brain

I'd love feedback, especially from embedded engineers and signal processing
folks. What temporal patterns would you want to detect?

---

## 2. Reddit r/rust Post

### Title: "CricketBrain v1.0 — neuromorphic signal processor in Rust: 0.175us/step, 944 bytes, no_std, 1 dependency"

I just released CricketBrain, a biomorphic inference engine modeled after the
cricket auditory system. It uses delay-line coincidence detection instead of
ML — 5 neurons, 6 synapses, zero training.

**Key facts:**
- `0.175 us/step` (Criterion-measured on x86-64)
- `944 bytes` calculated RAM (no_std, no heap)
- `1 dependency` in core crate (libm)
- `#![deny(unsafe_code)]` in core
- `85 tests`, CI on Linux/macOS/Windows
- Bindings: C FFI, Python (PyO3), WASM

**What I'd appreciate feedback on:**
- API design — does `cricket_brain::prelude::*` make sense?
- The adaptive bandwidth mechanism for dense token vocabularies
- Whether the `no_std` architecture is viable for real Cortex-M targets

Repo: https://github.com/BEKO2210/cricket-brain
Whitepaper: https://github.com/BEKO2210/cricket-brain/blob/main/RESEARCH_WHITEPAPER.md

Built with AI-assisted development (Claude Code, ChatGPT/Codex, Kimi, Gemini)
— full transparency statement in the repo.

---

## 3. Reddit r/embedded Post

### Title: "no_std Rust neuromorphic engine: 944 bytes, 1 dep (libm), deny(unsafe_code) — would this work on Cortex-M?"

Working on CricketBrain, a signal processor inspired by cricket hearing. The
core crate is `no_std` with a single dependency (`libm`) and
`#![deny(unsafe_code)]`.

I've designed it for embedded but only verified on host. The `arduino_minimal`
example uses fixed-size arrays (no heap) and calculates to 944 bytes RAM.

**Honest question:** Has anyone here tried deploying a similar Rust `no_std`
library on actual Cortex-M0/M4 hardware? What gotchas should I expect?

The architecture: 5 neurons with Gaussian frequency selectivity, 6 delay-line
synapses, coincidence detection gate. Processes one step in ~0.175us on x86.

Repo: https://github.com/BEKO2210/cricket-brain

---

## 4. Hacker News Submission

### Title: CricketBrain: Neuromorphic signal processor in Rust (0.175us/step, 944 bytes)

### URL: https://github.com/BEKO2210/cricket-brain

---

## 5. Twitter/X Thread

**Tweet 1:**
Just released CricketBrain v1.0 — a neuromorphic signal processor in Rust
inspired by cricket hearing.

5 neurons. 6 synapses. 0.175 us/step. 944 bytes RAM. Zero training.

No CUDA. No weights. No matrices. Just biology turned into code.

https://github.com/BEKO2210/cricket-brain

Thread (1/5)

**Tweet 2:**
How does it work?

A cricket finds mates using delay-line coincidence detection — signals
travel through neurons with different delays, and the output fires only
when timing matches.

CricketBrain implements this exact circuit in safe Rust (no_std, 1 dep).

(2/5)

**Tweet 3:**
Benchmarks vs classical detectors (SNR = 0 dB):

CricketBrain: TPR 1.0, FPR 0.0
IIR Bandpass: TPR 1.0, FPR 0.56
Matched Filter: TPR 0.0, FPR 0.0

The coincidence gate rejects noise that frequency-only methods can't.

(Note: synthetic benchmark — real-world will vary)

(3/5)

**Tweet 4:**
The stack:
- Rust workspace (core/brain/ffi/python/wasm)
- #![deny(unsafe_code)] in core
- AGPL-3.0 + commercial license
- 85 tests, CI on 3 platforms
- Whitepaper with 16 references + ablation study

Built with AI-assisted development — full transparency in the repo.

(4/5)

**Tweet 5:**
Use cases I'm exploring:
- Embedded sensor pattern detection (IoT)
- Audio temporal analysis
- Research tool for neuromorphic computing

Looking for feedback from embedded engineers and signal processing folks.

What would you build with sub-microsecond pattern detection?

(5/5)

---

## 6. LinkedIn Post

**CricketBrain v1.0 — Open Source Neuromorphic Signal Processor**

I'm excited to share CricketBrain, a project I've been developing that
takes a fundamentally different approach to signal processing.

Instead of machine learning, CricketBrain uses delay-line coincidence
detection — the same mechanism field crickets use to recognize songs.
The result is a signal processor that runs in 0.175 microseconds per step
with 944 bytes of RAM.

Key technical highlights:
- Rust implementation with no_std core (1 dependency)
- Bindings for C/C++, Python, and WebAssembly
- 85 tests, CI on Linux/macOS/Windows
- Research whitepaper with 16 peer-reviewed references
- AGPL-3.0 open source + commercial licensing

Potential application areas: embedded IoT sensing, audio pattern detection,
industrial monitoring, and neuromorphic computing research.

The project was developed with AI-assisted tools (Claude Code, ChatGPT/Codex,
Kimi, Gemini) — I believe in full transparency about the development process.

I'm looking for feedback from embedded systems engineers, signal processing
experts, and anyone working in edge computing.

GitHub: https://github.com/BEKO2210/cricket-brain

#Rust #OpenSource #Embedded #NeuromorphicComputing #SignalProcessing
#EdgeAI #IoT

---

## 7. Rust Weekly Newsletter Submission

**Email to:** thisweekinrust@gmail.com

**Subject:** CricketBrain v1.0 — neuromorphic signal processor (no_std, 1 dep, deny(unsafe_code))

Hi,

I'd like to submit CricketBrain for This Week in Rust:

- **Category:** Crate of the Week / Interesting Projects
- **URL:** https://github.com/BEKO2210/cricket-brain
- **Description:** Biomorphic signal processor inspired by cricket auditory
  circuits. Uses delay-line coincidence detection for temporal pattern
  recognition. no_std core with 1 dependency (libm), #![deny(unsafe_code)],
  0.175us/step latency. Includes C FFI, Python, and WASM bindings.
- **License:** AGPL-3.0 + Commercial

Thank you!
Belkis Aslani
