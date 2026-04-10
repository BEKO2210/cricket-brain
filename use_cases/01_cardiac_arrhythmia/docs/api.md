# UC01 Cardiac Arrhythmia — API Reference

**Date:** 2026-04-10 | **Crate:** `cricket-brain-cardiac v0.1.0`

---

## CardiacDetector

The main entry point. Wraps a `CricketBrain` instance with RR-interval tracking
and rhythm classification logic.

### Construction

```rust
use cricket_brain_cardiac::detector::CardiacDetector;

let mut detector = CardiacDetector::new();
// Internally creates BrainConfig with:
//   seed = 42
//   adaptive_sensitivity = true
//   privacy_mode = true
```

### Core Method: `step()`

```rust
pub fn step(&mut self, input_freq: f32) -> Option<RhythmClass>
```

Feed one frequency sample (1 ms timestep). Returns `Some(RhythmClass)` when
a new classification is available (typically after a QRS burst ends and
enough RR intervals are accumulated).

| Parameter | Type | Description |
|-----------|------|-------------|
| `input_freq` | `f32` | Frequency in Hz (0.0 = silence, 4500.0 = QRS) |
| **Returns** | `Option<RhythmClass>` | Classification when available |

### Batch Method: `classify_stream()`

```rust
pub fn classify_stream(&mut self, beats: &[BeatRecord]) -> Vec<BeatClassification>
```

Process an entire sequence of `BeatRecord`s. Resets internal state, converts
beats to frequency stream, and collects all classifications.

### Query Methods

```rust
pub fn confidence(&self) -> f32        // 0.0–1.0, last classification confidence
pub fn bpm_estimate(&self) -> f32      // Current BPM from mean RR interval
pub fn last_classification(&self) -> Option<RhythmClass>
pub fn steps_processed(&self) -> usize // Total timesteps fed
pub fn memory_usage_bytes(&self) -> usize  // CricketBrain heap RAM (928 bytes)
pub fn reset(&mut self)                // Zero all state
```

---

## RhythmClass

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmClass {
    NormalSinus,    // 60–100 BPM, CV < 0.3
    Tachycardia,    // > 100 BPM
    Bradycardia,    // < 60 BPM
    Irregular,      // CV > 0.3 (high RR variability)
}
```

Implements `Display`:
- `NormalSinus` → `"Normal Sinus"`
- `Tachycardia` → `"Tachycardia"`
- etc.

---

## BeatClassification

Returned by `classify_stream()`:

```rust
pub struct BeatClassification {
    pub rhythm: RhythmClass,  // Classification result
    pub confidence: f32,      // 0.0–1.0
    pub bpm: f32,             // Estimated BPM at this beat
    pub step: usize,          // Timestep when classification occurred
}
```

---

## ConfusionMatrix

```rust
pub struct ConfusionMatrix {
    pub tp_normal: usize,
    pub fp_normal: usize,
    pub fn_normal: usize,
    pub tp_tachy: usize,
    pub fp_tachy: usize,
    pub fn_tachy: usize,
    pub tp_brady: usize,
    pub fp_brady: usize,
    pub fn_brady: usize,
    pub total: usize,
    pub correct: usize,
}
```

### Construction

```rust
let cm = ConfusionMatrix::from_predictions(&classifications, &beats);
```

Ground truth is derived from the detector's BPM estimate at each prediction.

### Methods

```rust
pub fn accuracy(&self) -> f32   // correct / total
pub fn print(&self)             // Formatted table to stdout
```

---

## EcgCycle

Synthetic ECG waveform for testing:

```rust
pub struct EcgCycle {
    pub segments: Vec<(f32, usize)>,  // (frequency_hz, duration_ms)
}
```

### Factory Functions

```rust
pub fn normal_sinus() -> EcgCycle   // ~73 BPM (RR = 820 ms)
pub fn tachycardia() -> EcgCycle    // ~150 BPM (RR = 400 ms)
pub fn bradycardia() -> EcgCycle    // ~40 BPM (RR = 1500 ms)
```

### Methods

```rust
pub fn duration_ms(&self) -> usize
pub fn bpm(&self) -> f32
pub fn to_frequency_stream(&self, n_cycles: usize) -> Vec<f32>
```

---

## CSV I/O

### BeatRecord

```rust
pub struct BeatRecord {
    pub timestamp_ms: f32,
    pub rr_interval_ms: f32,
    pub beat_type: String,      // AAMI: "N", "S", "V", "F", "Q"
    pub bpm: f32,
    pub mapped_freq: f32,
}
```

### Functions

```rust
pub fn from_csv(path: &str) -> Vec<BeatRecord>
pub fn beats_to_frequency_stream(beats: &[BeatRecord]) -> Vec<f32>
pub fn write_sample_csv(path: &str, n_per_class: usize)
```

---

## Confidence Model

```
confidence = data_factor × 0.4 + stability_factor × 0.6

where:
  data_factor     = min(n_intervals / window_size, 1.0)
  stability_factor = max(1.0 - CV, 0.0)
  CV              = std(RR) / mean(RR)
  window_size     = 8 intervals
```

Confidence increases with more data (up to 8 beats) and decreases with
higher RR variability. Range: [0.0, 1.0].

---

## Classification Thresholds

| Rhythm | BPM Range | CV Threshold |
|--------|-----------|-------------|
| Bradycardia | < 60 | — |
| Normal Sinus | 60–100 | < 0.3 |
| Tachycardia | > 100 | — |
| Irregular | any | > 0.3 |

Note: CV (coefficient of variation) is checked first. If CV > 0.3, the
rhythm is classified as Irregular regardless of BPM.
