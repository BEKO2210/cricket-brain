# UC02 Predictive Maintenance — API Reference

**Date:** 2026-04-10 | **Crate:** `cricket-brain-bearings v0.1.0`

---

## BearingDetector

4-channel fault detector wrapping a CricketBrain `ResonatorBank`.

### Construction

```rust
let mut det = BearingDetector::new();
// Creates ResonatorBank with 4 channels:
//   FTF (15 Hz), BSF (69 Hz), BPFO (107 Hz), BPFI (162 Hz)
// 20 neurons, 3712 bytes RAM
```

### Speed Compensation

```rust
det.set_rpm(900.0);   // Scale input frequencies to calibration RPM
det.clear_rpm();       // Disable compensation
```

Formula: `f_compensated = f_input × (1797.0 / current_rpm)`

### Core: `step()`

```rust
pub fn step(&mut self, input_freq: f32) -> Option<FaultType>
```

Feed one vibration frequency sample. Returns classification at end of each
50-step detection window.

### Batch: `classify_stream()`

```rust
pub fn classify_stream(
    &mut self,
    windows: &[VibrationWindow],
    steps_per_window: usize,
) -> Vec<WindowClassification>
```

### Query Methods

```rust
pub fn confidence(&self) -> f32
pub fn last_fault(&self) -> FaultType
pub fn steps_processed(&self) -> usize
pub fn total_neurons(&self) -> usize     // 20
pub fn memory_usage_bytes(&self) -> usize // 3712
pub fn reset(&mut self)
```

---

## FaultType

```rust
pub enum FaultType {
    Normal,      // No fault energy detected
    OuterRace,   // BPFO channel dominant (107 Hz)
    InnerRace,   // BPFI channel dominant (162 Hz)
    BallDefect,  // BSF channel dominant (69 Hz)
}
```

---

## WindowClassification

```rust
pub struct WindowClassification {
    pub fault: FaultType,
    pub confidence: f32,  // 0.0–1.0 (dominance ratio)
    pub step: usize,
}
```

---

## ConfusionMatrix

```rust
let cm = ConfusionMatrix::from_predictions(&preds, &windows, steps_per_window);
cm.print();       // Formatted 4-class table
cm.accuracy();    // correct / total
```

---

## VibrationWindow (CSV I/O)

```rust
pub struct VibrationWindow {
    pub timestamp_ms: f32,
    pub dominant_freq: f32,
    pub amplitude: f32,
    pub fault_label: String,  // "Normal", "OR", "IR", "Ball"
}

pub fn from_csv(path: &str) -> Vec<VibrationWindow>
pub fn windows_to_frequency_stream(windows: &[VibrationWindow], steps: usize) -> Vec<f32>
```

---

## Signal Generators

```rust
pub fn normal_vibration(n_steps: usize) -> Vec<f32>
pub fn outer_race_fault(n_steps: usize) -> Vec<f32>  // BPFO 107 Hz
pub fn inner_race_fault(n_steps: usize) -> Vec<f32>  // BPFI 162 Hz
pub fn ball_fault(n_steps: usize) -> Vec<f32>         // BSF 69 Hz
```

---

## Detection Logic

1. Input frequency → speed compensation (if RPM set)
2. Compensated frequency → ResonatorBank (4 parallel 5-neuron circuits)
3. Each channel outputs spike amplitude per step
4. Energy accumulated over 50-step window per channel
5. At window end: `argmax(channel_energy)` → FaultType
6. Confidence = `max_energy / total_energy`
7. If `total_energy < 0.1` → Normal (no fault)
