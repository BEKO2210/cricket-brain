# UC04 Power Grid — API Reference

**Date:** 2026-04-24 | **Crate:** `cricket-brain-grid v0.1.0`

---

## GridDetector

4-channel power-grid event triage core wrapping a CricketBrain
`ResonatorBank`.

### Construction

```rust
use cricket_brain_grid::detector::GridDetector;

// v0.1 — strict tuning (bandwidth ~0.10 auto-clamped)
let mut det = GridDetector::new();

// v0.2 — wide tuning (recommended bandwidth 0.20) + multi-label path
let mut det = GridDetector::with_bandwidth(0.20);
```

Creates a `ResonatorBank` with 4 channels at 50 / 100 / 150 / 200 Hz
(integer multiples of the 50 Hz fundamental). 20 neurons, 3,712 bytes
RAM.

### Outage & Channel Thresholds

```rust
det.set_outage_threshold(0.10);   // total energy → Outage if below
det.set_channel_threshold(0.03);  // per-channel multi-label threshold
det.set_bandwidth(0.20);          // widen tuning to recover boundaries
```

### Core: `step()`

```rust
pub fn step(&mut self, input_freq: f32) -> Option<GridEvent>
```

Feed one PMU dominant-frequency sample (Hz). Returns a classification
at the end of each 50-step window.

### v0.2 Multi-Label: `step_multi()`

```rust
pub fn step_multi(&mut self, input_freq: f32) -> Option<MultiLabelDecision>

pub struct MultiLabelDecision {
    pub events: Vec<GridEvent>,  // every channel above channel_threshold
    pub energies: [f32; 4],       // FUND, H2, H3, H4 snapshot
    pub step: usize,
}
```

Reports **every** channel above the per-channel threshold — useful for
the typical mixed grid where 50 Hz fundamental coexists with 3rd-
harmonic distortion from connected non-linear loads.

### Batch: `classify_stream()`

```rust
pub fn classify_stream(
    &mut self,
    windows: &[GridWindow],
    steps_per_window: usize,
) -> Vec<WindowClassification>
```

### Query Methods

```rust
pub fn confidence(&self) -> f32
pub fn last_event(&self) -> GridEvent
pub fn steps_processed(&self) -> usize
pub fn total_neurons(&self) -> usize      // 20
pub fn memory_usage_bytes(&self) -> usize // 3712
pub fn reset(&mut self)
```

---

## GridEvent

```rust
pub enum GridEvent {
    Outage,           // total channel energy below outage_threshold
    Nominal,          // 50 Hz fundamental dominant
    SecondHarmonic,   // 100 Hz dominant — DC offset / saturation / asymmetry
    ThirdHarmonic,    // 150 Hz dominant — non-linear loads (VFDs, SMPS, LED)
    FourthHarmonic,   // 200 Hz dominant — switching artefacts, fast EMI
}
```

---

## ConfusionMatrix

```rust
let cm = ConfusionMatrix::from_predictions(&preds, &windows, steps_per_window);
cm.print();         // formatted 5-class table
cm.accuracy();      // correct / total
```

---

## GridWindow (CSV I/O)

```rust
pub struct GridWindow {
    pub timestamp_ms: f32,
    pub dominant_freq: f32,
    pub thd_pct: f32,         // total harmonic distortion (informative)
    pub event_label: String,  // Outage | Nominal | SecondHarmonic | ThirdHarmonic | FourthHarmonic
}

pub fn from_csv(path: &str) -> Vec<GridWindow>
pub fn windows_to_frequency_stream(
    windows: &[GridWindow],
    steps_per_window: usize,
) -> Vec<f32>
```

---

## Signal Generators

```rust
pub fn nominal_grid(n_steps: usize) -> Vec<f32>
pub fn outage(n_steps: usize) -> Vec<f32>
pub fn second_harmonic_dominant(n_steps: usize) -> Vec<f32>  // 100 Hz
pub fn third_harmonic_dominant(n_steps: usize) -> Vec<f32>   // 150 Hz
pub fn fourth_harmonic_dominant(n_steps: usize) -> Vec<f32>  // 200 Hz

/// Nominal grid → 3rd-harmonic burst (VFD load) → recovery.
pub fn factory_startup(n_steps: usize, disturbance_steps: usize) -> Vec<f32>

/// Nominal grid with several brief outage windows (load-shedding).
pub fn rolling_brownout(n_steps: usize, n_dips: usize, dip_len: usize) -> Vec<f32>

/// 70 % fundamental + 30 % 3rd-harmonic mix (typical building feed).
pub fn nominal_with_third_harmonic(n_steps: usize) -> Vec<f32>
```

---

## Detection Logic

1. Input frequency (Hz) from PMU FFT peak.
2. `ResonatorBank` emits per-channel spike amplitude.
3. Channel energy accumulated over 50-step window.
4. If total energy `< outage_threshold` → `Outage`.
5. Otherwise `argmax(channel_energy)` → `GridEvent`.
6. `confidence = max_energy / total_energy`.
