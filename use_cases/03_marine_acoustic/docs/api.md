# UC03 Marine Acoustic — API Reference

**Date:** 2026-04-24 | **Crate:** `cricket-brain-marine v0.1.0`

---

## MarineDetector

4-channel marine acoustic classifier wrapping a CricketBrain `ResonatorBank`.

### Construction

```rust
use cricket_brain_marine::detector::MarineDetector;

let mut det = MarineDetector::new();
// Creates a ResonatorBank with 4 channels:
//   FIN  (20 Hz, fin whale 20-Hz pulse)
//   BLUE (80 Hz, blue whale A-call tonal)
//   SHIP (140 Hz, cargo-ship cavitation peak)
//   HUMP (200 Hz, humpback song unit)
// 20 neurons, 3712 bytes RAM
```

### Ambient-Threshold Tuning

```rust
det.set_ambient_threshold(0.25);   // explicit numerical threshold
det.set_sea_state(6);              // Douglas scale 0-9; raises threshold 25 %/step
```

Use `set_sea_state` when the hydrophone is in rough seas (wind, waves,
breaking whitecaps). A higher sea state raises the detection threshold
so that loud broadband background does not trigger false whale alarms.

### Core: `step()`

```rust
pub fn step(&mut self, input_freq: f32) -> Option<AcousticEvent>
```

Feed one dominant-frequency sample (Hz) from a hydrophone FFT window.
Returns a classification at the end of each 50-step detection window
(nothing otherwise).

### Batch: `classify_stream()`

```rust
pub fn classify_stream(
    &mut self,
    windows: &[AcousticWindow],
    steps_per_window: usize,
) -> Vec<WindowClassification>
```

Consume a sequence of preprocessed CSV rows. Each window's
`dominant_freq` is repeated for `steps_per_window` timesteps.

### Query Methods

```rust
pub fn confidence(&self) -> f32
pub fn last_event(&self) -> AcousticEvent
pub fn steps_processed(&self) -> usize
pub fn total_neurons(&self) -> usize      // 20
pub fn memory_usage_bytes(&self) -> usize // 3712
pub fn reset(&mut self)
```

---

## AcousticEvent

```rust
pub enum AcousticEvent {
    Ambient,    // total channel energy below ambient_threshold
    FinWhale,   // FIN channel dominant (~20 Hz)
    BlueWhale,  // BLUE channel dominant (~80 Hz)
    ShipNoise,  // SHIP channel dominant (~140 Hz)
    Humpback,   // HUMP channel dominant (~200 Hz)
}
```

---

## WindowClassification

```rust
pub struct WindowClassification {
    pub event: AcousticEvent,
    pub confidence: f32,  // 0.0-1.0 (dominance ratio)
    pub step: usize,
}
```

---

## ConfusionMatrix

```rust
let cm = ConfusionMatrix::from_predictions(&preds, &windows, steps_per_window);
cm.print();       // formatted 5-class table
cm.accuracy();    // correct / total
```

---

## AcousticWindow (CSV I/O)

```rust
pub struct AcousticWindow {
    pub timestamp_ms: f32,
    pub dominant_freq: f32,
    pub rms_db: f32,
    pub event_label: String,  // "Ambient" | "FinWhale" | "BlueWhale" | "Ship" | "Humpback"
}

pub fn from_csv(path: &str) -> Vec<AcousticWindow>
pub fn windows_to_frequency_stream(
    windows: &[AcousticWindow],
    steps_per_window: usize,
) -> Vec<f32>
```

---

## Signal Generators

```rust
pub fn ambient_noise(n_steps: usize) -> Vec<f32>
pub fn fin_whale_call(n_steps: usize) -> Vec<f32>  // 20 Hz pulse train
pub fn blue_whale_call(n_steps: usize) -> Vec<f32> // 80 Hz tonal A-call
pub fn ship_passage(n_steps: usize) -> Vec<f32>    // 140 Hz sustained cavitation
pub fn humpback_song(n_steps: usize) -> Vec<f32>   // 200 Hz song units

/// Triangular-presence ship transit (CPA at n_steps / 2).
pub fn ship_transit(n_steps: usize) -> Vec<f32>

/// Fin-whale pulses mixed with a simultaneous ship passage.
pub fn fin_whale_under_ship(n_steps: usize) -> Vec<f32>
```

---

## Detection Logic

1. Input frequency (Hz) from hydrophone FFT peak.
2. `ResonatorBank` emits per-channel spike amplitude.
3. Channel energy accumulated over 50-step window.
4. If total energy `< ambient_threshold` → `Ambient`.
5. Otherwise `argmax(channel_energy)` → `AcousticEvent`.
6. `confidence = max_energy / total_energy`.
