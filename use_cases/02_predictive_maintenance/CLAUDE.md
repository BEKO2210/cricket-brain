# UC02 Predictive Maintenance — CLAUDE Build Plan

> Local plan file for Use Case 02. Updated after every run.

---

## 1. Overview

Bearing fault detection in rotating machinery via vibration frequency analysis.
The CWRU Bearing Dataset provides 12 kHz accelerometer data from a motor test
rig with induced faults (inner race, outer race, ball defects).

CricketBrain detects characteristic fault frequencies using Gaussian-tuned
resonators — one 5-neuron circuit per fault type. No training data needed,
runs on a $0.50 microcontroller.

---

## 2. Dataset: CWRU Bearing Data Center

| Field | Value |
|-------|-------|
| Source | Case Western Reserve University |
| URL | https://engineering.case.edu/bearingdatacenter |
| License | Public Domain |
| Sampling | 12,000 Hz and 48,000 Hz drive-end accelerometer |
| Fault types | Inner race, outer race, ball, normal |
| Fault sizes | 0.007", 0.014", 0.021" diameter |
| Motor loads | 0-3 HP |
| Format | .mat (MATLAB) files |

### Key Frequencies (SKF 6205-2RS bearing)
- Ball Pass Frequency Outer (BPFO): ~107 Hz
- Ball Pass Frequency Inner (BPFI): ~162 Hz
- Ball Spin Frequency (BSF): ~69 Hz
- Fundamental Train Frequency (FTF): ~15 Hz

---

## 3. CricketBrain Approach

Use a **ResonatorBank** with 4 channels tuned to the 4 fault frequencies.
Each channel is a 5-neuron circuit. When a fault frequency is sustained,
the coincidence gate fires → fault detected.

```rust
let vocab = TokenVocabulary::new(
    &["BPFO", "BPFI", "BSF", "FTF"],
    100.0,  // min freq
    200.0,  // max freq
);
let mut bank = ResonatorBank::new(&vocab);
let outputs = bank.step(input_freq);
// outputs[0] > 0 → outer race fault
// outputs[1] > 0 → inner race fault
// etc.
```

---

## 4. Ten-Run Plan

| Run | Deliverable | Status |
|-----|-------------|--------|
| 1 | DONE | 2026-04-10 | Scaffold, 9/9 tests, 4 fault types detected, 0.127-0.285 µs/step |
| 2 | DONE | 2026-04-10 | Python FFT pipeline, CSV I/O, 10/10 tests, 200 windows |
| 3 | DONE | 2026-04-10 | CSV integration, ConfusionMatrix, 12/12 tests, 93.0% accuracy |
| 4 | DONE | 2026-04-10 | SDT d'=6.18 all EXCELLENT, 0.129-0.264 µs/step, 3712B/20N |
| 5 | DONE | 2026-04-10 | evaluate.py (F1=0.932), 3 plots, docs/results.md |
| 6 | DONE | 2026-04-10 | 100% noise-robust, speed-comp FIXED (6/6 RPMs) |
| 7 | DONE | 2026-04-10 | website/pages/bearings.html, nav dropdown+card linked |
| 8 | DONE | 2026-04-10 | Full README with real results, docs/api.md reference |
| 9 | CI integration | PENDING |
| 10 | Metrics finalization | PENDING |

---

## 5. Next Prompt

--- NEXT PROMPT START ---
Lies use_cases/02_predictive_maintenance/CLAUDE.md und fuehre Run 1 aus.

Run 1 Deliverables:
1. Erstelle use_cases/02_predictive_maintenance/Cargo.toml:
   - name = "cricket-brain-bearings"
   - version = "0.1.0", edition = "2021", rust-version = "1.75"
   - license = "AGPL-3.0-only"
   - dependency: cricket-brain = { path = "../.." }
   - [workspace] (standalone)

2. Erstelle use_cases/02_predictive_maintenance/src/lib.rs:
   - pub mod detector;
   - pub mod vibration_signal;

3. Erstelle use_cases/02_predictive_maintenance/src/vibration_signal.rs:
   - Bearing fault frequencies (BPFO=107, BPFI=162, BSF=69, FTF=15 Hz)
   - pub fn normal_vibration(n_steps: usize) -> Vec<f32> (baseline, no faults)
   - pub fn outer_race_fault(n_steps: usize) -> Vec<f32> (BPFO dominant)
   - pub fn inner_race_fault(n_steps: usize) -> Vec<f32> (BPFI dominant)
   - pub fn ball_fault(n_steps: usize) -> Vec<f32> (BSF dominant)

4. Erstelle use_cases/02_predictive_maintenance/src/detector.rs:
   - pub struct BearingDetector mit ResonatorBank (4 Channels)
   - pub enum FaultType { Normal, OuterRace, InnerRace, BallDefect }
   - pub fn step(&mut self, freq: f32) -> Option<FaultType>
   - pub fn confidence(&self) -> f32

5. Erstelle use_cases/02_predictive_maintenance/src/main.rs:
   - Demo: Normal + 3 Fault-Typen, Klassifikation ausgeben

6. Erstelle use_cases/02_predictive_maintenance/data/SOURCES.md

7. Verifiziere: cargo build, cargo run, cargo test

8. Update CLAUDE.md: Run 1 = DONE, NEXT PROMPT fuer Run 2

REGELN:
- Aendere NICHTS ausserhalb von use_cases/ (ausser Website-Links wenn noetig)
- Commit und push am Ende
--- NEXT PROMPT END ---
