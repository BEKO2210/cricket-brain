# Projekt-Dossier: CricketBrain (Silent Sentinel)

**Status:** Released — `v1.0.0`
**Vision:** *"The Java of Sensing"*
**Rust Edition:** 2021 | **MSRV:** 1.75+
**Lizenz:** AGPL-3.0 + Commercial (see [COMMERCIAL.md](COMMERCIAL.md))

---

## 1. Kurzuebersicht

CricketBrain ist ein hochperformanter, biologisch inspirierter Signalprozessor. Er nutzt Resonatoren, Verzoegerungs-Synapsen und Koinzidenz-Detektion, um zeitliche Muster in Rohdaten (z. B. EKG, Audio, Sensoren) zu erkennen.

| Metrik | Wert |
|--------|------|
| **Performance** | ~0.175 us pro Simulationsschritt (5-Neuron kanonisch) |
| **Throughput** | 3.43 x 10^7 Neuron-Ops/sec (40k-Neuron Skalierung) |
| **Footprint** | `no_std` Kern, minimaler RAM-Bedarf (944 Bytes Arduino-Minimal, < 64 KB Planungslimit) |
| **Impact** | Medizinische Ueberwachung (Sentinel), industrielle Wartung, IoT-Sicherheit |

---

## 2. Architektur (Workspace-Struktur)

Das Projekt ist als Rust-Workspace organisiert, um strikte Trennung zwischen Kernlogik und Anwendung zu gewaehrleisten:

```
cricket-brain/                    # Workspace Root (v1.0.0)
|
|-- crates/core/                  # [1] Das wissenschaftliche Herz (no_std)
|-- src/                          # [2] Die Anwendungs-Logik (brain, sequence, tokens)
|-- crates/ffi/                   # [3] C-kompatible Bruecke (cdylib + staticlib)
|-- crates/python/                # [4] PyO3-Bindings
|-- crates/wasm/                  # [5] wasm-bindgen Browser-Integration
|-- examples/                     # [6] Referenz-Implementierungen
|-- benches/                      # [7] Criterion-Benchmarks
|-- tests/                        # [8] Integrationstests
`-- docs/                         # [9] Dokumentation
```

### [crates/core] — Das wissenschaftliche Herz

`#![no_std]` | `#![deny(unsafe_code)]` | Einzige Abhaengigkeit: `libm 0.2`

| Datei | Zeilen | Beschreibung |
|-------|--------|--------------|
| `neuron.rs` | 270 | Gausssche Frequenzselektivitaet + Phasen-Locking Resonator. Mathematisches Modell: `R(f, f0, w) = exp(-(df / f0 / w)^2)` mit w=0.1 (10% Bandbreite). Koinzidenz-Gate: `A(t) > theta AND A(t-tau) > theta x 0.8` |
| `synapse.rs` | 146 | Ringbuffer-basierte Signalverzoegerung (VecDeque). Axonale Delay-Simulation mit inhibitorischer Inversion. `#[inline(always)]` auf `transmit()` |
| `memory.rs` | 24 | Statisches & dynamisches Memory-Accounting (`MemoryStats`). Konstante: `EMBEDDED_RAM_LIMIT_BYTES = 64 * 1024` |
| `logger.rs` | 138 | `no_std` Telemetrie-Traits (`CricketLogger`, `Telemetry`). Events: Spike, ResonanceChange, SequenceMatched, SnrReport, SystemOverload |
| `error.rs` | 28 | `CricketError` Enum: InvalidConfiguration, TokenNotFound, InvalidInput |

**Kern-Structs:**

```rust
// Neuron — Resonator mit Delay-Line History
pub struct Neuron {
    id: usize,
    eigenfreq: f32,          // Abgestimmte Frequenz (Hz)
    phase: f32,              // Oszillations-Phase (0-1)
    amplitude: f32,          // Aktivierungslevel (0-1)
    threshold: f32,          // Feuerungsschwelle (default: 0.7)
    delay_taps: usize,       // Koinzidenz-Fenster (ms)
    history: VecDeque<f32>,  // Ringbuffer fuer verzoegerte Samples
    min_activation_threshold: f32, // Squelch-Floor (CFAR-Stil)
}

// DelaySynapse — Axonale Verzoegerung
pub struct DelaySynapse {
    from: usize,             // Quell-Neuron Index
    to: usize,               // Ziel-Neuron Index
    delay_ms: usize,         // Propagationsverzoegerung
    inhibitory: bool,        // Signal-Inversion
    ring_buffer: VecDeque<f32>,
}
```

### [src/] — Die Anwendungs-Logik

| Datei | Zeilen | Beschreibung |
|-------|--------|--------------|
| `lib.rs` | 73 | Oeffentliche API, Prelude, FFI-Fehlercodes (`CRICKET_OK=0` bis `CRICKET_ERR_INTERNAL=255`) |
| `main.rs` | 60 | SOS Morse Demo Binary (Default `cargo run` Einstiegspunkt) |
| `brain.rs` | 876 | Haupt-Netzwerk-Steuerung (`CricketBrain`). Muenster-Modell: AN1->LN2->LN3->LN5->ON1 Schaltkreis |
| `sequence.rs` | 680 | Mustererkennung mit Confidence-Scoring: `C = SNR * (1 - jitter/tolerance)`. N-Gram Pattern-Matching |
| `resonator_bank.rs` | 327 | Parallelisierbare Frequenzueberwachung (optionales Rayon). Ein 5-Neuron-Kanal pro Token |
| `token.rs` | 221 | Multi-Frequenz Token-Vokabular (v0.2). Alphabet: 27 Tokens, 2000-8000 Hz |
| `patterns.rs` | 297 | Morse-Code Encoding/Decoding. DOT=50ms, DASH=150ms bei 4500 Hz |
| `json_telemetry.rs` | 139 | JSONL Echtzeit-Datenstrom (erfordert `cli` Feature) |

**Muenster-Modell (Kanonischer 5-Neuron Schaltkreis):**

```
         AN1 (id=0, Rezeptor, 4 taps)
        / | \
       /  |  \
      v   v   v
    LN2  LN3  LN5
   (inh) (exc) (inh)
   3ms   2ms   5ms
      \   |   /
       \  |  /
        v v v
     ON1 (id=4, Ausgang, Koinzidenz-Gate)
```

**Synapsen:** AN1->LN2 (3ms, inh), AN1->LN3 (2ms, exc), AN1->LN5 (5ms, inh), LN2->ON1 (1ms, inh), LN3->ON1 (1ms, exc), LN5->ON1 (1ms, inh)

### [crates/ffi] — C-kompatible Bruecke

| Export | Signatur |
|--------|----------|
| `brain_new` | `(out_handle, n_neurons, min_freq, max_freq) -> i32` |
| `brain_step` | `(handle, input_freq, out_output) -> i32` |
| `brain_get_status` | `(handle, out_status) -> i32` |
| `brain_free` | `(handle)` — Deallokation |
| `brain_get_version` | `() -> *const c_char` |

Header: `crates/ffi/include/cricket_brain.h` (C, C++, Swift kompatibel)

### [crates/python] — PyO3-Bindings

```python
from cricket_brain import BrainConfig, Brain

config = BrainConfig()
config.n_neurons = 5
config.privacy_mode = True

brain = Brain(config)
output = brain.step(4500.0)
batch = brain.step_batch([4500.0, 0.0, 4500.0])
brain.reset()
```

Build: `cd crates/python && maturin develop`

### [crates/wasm] — Browser-Integration

```typescript
import { Brain } from "cricket-brain-wasm";

const brain = new Brain(42);         // seed
const output = brain.step(4500.0);
const events = brain.drainTelemetry();
const pred = brain.latestPrediction();
brain.reset();
```

Build: `cd crates/wasm && wasm-pack build --target web --out-dir pkg`

---

## 3. Wichtige Dateien & Einstiegspunkte

| Zweck | Datei |
|-------|-------|
| **Haupteinstieg** | `src/lib.rs` → nutze `cricket_brain::prelude` |
| **Wissenschaftlicher Beweis** | `RESEARCH_WHITEPAPER.md` (Mathematik & Ergebnisse) |
| **Medizinisches Beispiel** | `examples/sentinel_ecg_monitor.rs` (Tachykardie-Erkennung) |
| **Forschungs-Tool** | `examples/research_gen.rs` (Parametrische SNR-Sweeps, ROC-Kurven) |
| **Betrieb/CLI** | `examples/cricket_cli.rs` (Snapshotting, Resume, Live-Streaming, TOML/JSON Config) |
| **Morse-Demo** | `examples/morse_alphabet.rs` + `examples/live_demo.rs` (Encode->Brain->Decode Roundtrip) |
| **Skalierungs-Test** | `examples/scale_test.rs` (40.960 Neuronen Durchsatz) |
| **Embedded-Referenz** | `examples/arduino_minimal.rs` (no_std, Fixed-Array, 944 Bytes RAM) |
| **Sequenz-Vorhersage** | `examples/sequence_predict.rs` + `examples/scale_predict.rs` (256-Token, 1280 Neuronen) |
| **Python-Integration** | `examples/python_sentinel.py` |
| **WASM-Demo** | `examples/wasm_demo/` (index.html + main.ts) |

---

## 4. Features & Compliance

### Feature-Flags

| Flag | Effekt |
|------|--------|
| `std` (default) | Standard-Library, Heap-Allokation |
| `no_std` | Embedded-Modus (benoetigt `alloc`) |
| `telemetry` | Strukturierte Event-Hooks |
| `serde` | Serialisierung fuer Snapshots |
| `parallel` | Rayon-basierte Parallelisierung (ResonatorBank) |
| `cli` | Command-Line Tools (JSON-Telemetrie, Config-Parsing) |

### Sicherheit & Compliance

| Aspekt | Implementation |
|--------|---------------|
| **Privacy Mode** | `BrainConfig::privacy_mode = true` — Anonymisierung von Zeitstempeln, Coarsening von Telemetrie-Werten (HIPAA/DSGVO) |
| **Trust Layer** | Confidence-Scoring (`C = SNR * (1 - jitter/tolerance)`) und Ueberlastungs-Erkennung (Shannon-Entropie > 3.2 + >80% aktive Neuronen) |
| **Persistenz** | Vollstaendiger Snapshot/Restore-Support (`BrainSnapshot`) mit CRC64-Checksummen + Versions-Hash |
| **Security** | `#![deny(unsafe_code)]` im Core-Crate. Minimale Abhaengigkeiten. `cargo audit` in CI |
| **Unsafe-Boundary** | Nur in FFI-Crate (`extern "C"` Funktionen), mit SAFETY-Kommentaren und Null-Checks |

### FFI-Fehlercodes (Sprachueber-greifender Vertrag)

```rust
pub const CRICKET_OK: i32 = 0;
pub const CRICKET_ERR_NULL: i32 = 1;
pub const CRICKET_ERR_INVALID_CONFIG: i32 = 2;
pub const CRICKET_ERR_TOKEN_NOT_FOUND: i32 = 3;
pub const CRICKET_ERR_INVALID_INPUT: i32 = 4;
pub const CRICKET_ERR_INTERNAL: i32 = 255;
```

---

## 5. Performance-Daten (Run 14, 2026-04-06)

| Szenario | Metrik |
|----------|--------|
| 5-Neuron kanonisch | **0.175 us/step** |
| 40.960-Neuron Skalierung | **3.43 x 10^7 neuron-ops/sec** |
| Arduino-Minimal (no_std) | **944 Bytes RAM** |
| 40k-Neuron Memory | **13.91 MB** |
| Sequenz-Predictor | **0.30 MB** |
| Zeitkomplexitaet | **O(N+S)** pro Step (N=Neuronen, S=Synapsen) |
| Speicherkomplexitaet | **O(N*H + S*D)** (H=History, D=Delay) |

---

## 6. Kern-Algorithmus: Processing Pipeline

Jeder `CricketBrain::step(input_freq)` Aufruf:

1. **Phase-Dither:** xorshift64* RNG generiert stochastische Phase
2. **Adaptive Sensitivity (AGC):** `g(t) = clamp(1.2 - 0.4 * E(t), 0.6, 1.4)` (EMA auf Input-Energie)
3. **Entropie-Update:** Shannon-Entropie ueber 16 Frequenz-Bins (128-Sample Fenster)
4. **AN1 Resonanz:** `neuron[0].resonate(input_freq, phase)` — Gausssche Frequenz-Selektivitaet
5. **Synaptische Transmission:** Verzoegerte Signale via Ringbuffer an Downstream-Neuronen
6. **Downstream-Update:** LN2, LN3, LN5 resonieren mit synaptischen Inputs
7. **Output-Gate:** ON1 Koinzidenz-Detektion — feuert nur wenn `A(t) > theta AND A(t-tau) > theta * 0.8`
8. **Zeitschritt:** `time_step += 1`

---

## 7. Quellenstatistik

| Bereich | Dateien | Zeilen |
|---------|---------|--------|
| Core (crates/core/src) | 6 | 617 |
| Brain (src/) | 8 | 2.673 |
| FFI (crates/ffi) | 1 | 164 |
| Python (crates/python) | 1 | 121 |
| WASM (crates/wasm) | 1 | 130 |
| **Gesamt Produktionscode** | **17** | **3.705** |
| Beispiele | 14 | ~2.500+ |

---

## 8. Handover: Naechste Schritte (Post v1.0.0)

1. ~~**Final Audit:** Pruefung der FFI-Speichersicherheit~~ — Erledigt (12 FFI-Tests).
2. **Performance-Profile:** Verifizierung des 0.175us-Ziels auf ARM-Cortex-M Hardware.
3. ~~**Commercial-Ready:** Finalisierung der `COMMERCIAL.md`~~ — Erledigt (AGPL + 4-Tier Commercial).
4. ~~**Release 1.0:** Entfernen des RC1-Tags~~ — Erledigt (v1.0.0 released).
5. **STDP Learning:** Spike-Timing Dependent Plasticity fuer Online-Gewichtsanpassung.
6. **Hardware Deployment:** RISC-V / ARM Cortex-M Portierung mit Echtzeit-ADC.

---

## 9. Hinweis fuer die KI

- Arbeite strikt nach dem `no_std`-Prinzip im Kern (`crates/core`). Keine `std`-Importe dort.
- Nutze die Telemetrie-Hooks (`Telemetry` Trait) fuer alle neuen Features.
- **Vertrauen (Confidence) ist wichtiger als reine Detektion.** SNR und Jitter immer mitfuehren.
- Alle Hot-Path Funktionen muessen `#[inline(always)]` tragen.
- Neue Neuronen/Synapsen muessen `calculate_memory_requirements()` implementieren.
- Feature-Flags respektieren: `#[cfg(feature = "...")]` fuer optionale Abhaengigkeiten.
- `#![deny(unsafe_code)]` im Core niemals aufheben — Unsafe nur im FFI-Crate.
