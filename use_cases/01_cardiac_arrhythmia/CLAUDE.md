# UC01 Cardiac Arrhythmia — CLAUDE Build Plan

> Local plan file for Use Case 01. Updated after every run.

---

## 0. Working rules (v0.2 hardening)

These rules apply to **every** future change in this use case:

1. **Benchmark-first, not marketing.** Treat UC01 as a research
   benchmark suite. Do not enlarge website claims, market sizes, or
   accuracy headlines without a corresponding result file in
   `results/` produced by a fresh `cargo run --release --example …
   --write` invocation.
2. **Never invent results.** No fake MIT-BIH numbers, no "improved
   accuracy" without a regenerated result file, no hand-edited CSV
   metrics. Hardcoded benchmark scores in docs are tolerated **only**
   when explicitly labelled "example" and tied to a documented seed
   and command.
3. **Truth-based metrics only.** New benchmarks must use
   `evaluate::run_and_score` (or an equivalent that takes external
   ground truth). The legacy `ConfusionMatrix::from_predictions`
   path is preserved for traceability and must not be re-introduced
   into v0.2-or-later result files.
4. **Document every benchmark change.** When code changes affect
   results, update at minimum:
   - `README.md` (sample tables and command lines)
   - `BENCHMARK_ROADMAP.md` (state transitions of milestones / claims)
   - `docs/methodology.md` (definitions, conventions)
   - `docs/limitations.md` (new failure modes)
   - `docs/results.md` (versioned legacy log)
5. **Conservative medical claims.** No diagnosis. No "FDA
   plausible". No "validated on patients". Triage / pre-screening /
   research only.
6. **Repository scope:** changes for UC01 stay inside
   `use_cases/01_cardiac_arrhythmia/`. Touching `website/`, root
   `README.md`, or `MASTER_PLAN.md` is allowed *only* to fix wrong /
   stale numbers or commands; do not enlarge marketing copy.

---

## v0.2 (2026-04-25) — Benchmark hardening

Status: **DONE** in this PR.

New modules:

- `src/metrics.rs` — 4-class confusion matrix, per-class P/R/F1/specificity,
  macro-F1, weighted-F1, balanced accuracy, reject-aware coverage curve.
- `src/synthetic.rs` — labelled synthetic generator with seedable
  variability (HRV, baseline wander, amplitude jitter, morphology
  jitter, missing QRS, motion-artifact bursts, in-band noise).
- `src/evaluate.rs` — pairs detector emissions with ground-truth segments.
- `src/baselines.rs` — `ThresholdBurstBaseline`, `FrequencyRuleBaseline`.
- `src/report.rs` — JSON / CSV writer with metadata header
  (`generated_at`, `git_commit`, `seed`, `synthetic_generator_version`,
  `limitations`).

New benches:

- `cardiac_eval` — truth-based 4-class metrics, JSON + CSV + failure
  cases markdown.
- `cardiac_stress_sweep` — 7-dimension stress sweeps, per-dimension CSVs.
- `cardiac_baselines` — CricketBrain vs both rule baselines on 5 scenarios.
- `cardiac_reject` — coverage / accuracy curve.
- `cardiac_mitbih` — MIT-BIH skeleton (refuses to publish "validated"
  numbers until real ingestion lands).

Documentation:

- `BENCHMARK_ROADMAP.md` (new, top-level)
- `docs/methodology.md` (new)
- `README.md` (rewritten benchmarks section, structure block, results table)
- `docs/results.md` (marked legacy, points to v0.2 outputs)
- `docs/limitations.md` (added v0.2 audit findings as § 0)

Tests: 31 unit tests pass after the change (was 17).

---

## 1. Analysis of examples/sentinel_ecg_monitor.rs

The existing demo (116 lines) does:

### Synthetic ECG Waveform
```
P wave:  3100 Hz, 12 ms (atrial depolarization)
QRS:     4500 Hz, 10 ms (ventricular depolarization — carrier-aligned!)
T wave:  3400 Hz, 14 ms (ventricular repolarization)
RR gap:  18 ms (tachycardia, 188 BPM) or 88 ms (normal, 73 BPM)
```

### Detection Logic
1. Feed cycle samples into `brain.step(freq)` — CricketBrain resonates on QRS (4500 Hz)
2. Track `signal_energy` (output during tone) vs `noise_energy` (output during gaps)
3. Compute `SNR = 10 × log10(signal / noise)`
4. Compute `confidence = 0.85 × rhythm_score + 0.15 × spike_score`
5. Emit physician alert if `pattern_id == 1 AND confidence > 0.95 AND SNR >= 12 dB`

### Key Insight
The demo cheats slightly: it hardcodes `confidence.max(0.97)` and `snr.max(14.0)` for tachycardia. The real UC01 must earn its metrics from actual data.

### Brain Config Used
```rust
BrainConfig::default()
    .with_seed(12)
    .with_adaptive_sensitivity(true)
    .with_privacy_mode(true)
```

---

## 2. CricketBrain APIs for ECG

### Primary: step-based detection
```rust
// Create brain tuned to QRS complex frequency
let config = BrainConfig::default()
    .with_freq_range(3000.0, 5000.0)  // Cover P-QRS-T range
    .with_adaptive_sensitivity(true)
    .with_privacy_mode(true)
    .with_seed(42);
let mut brain = CricketBrain::new(config)?;

// Feed one sample per timestep
let output = brain.step(input_freq);  // Returns spike amplitude (0.0 or positive)
```

### Telemetry for clinical events
```rust
struct CardiacTelemetry { /* custom fields */ }
impl Telemetry for CardiacTelemetry {
    fn on_event(&mut self, event: TelemetryEvent) {
        match event {
            TelemetryEvent::Spike { neuron_id, timestamp } => { /* QRS detected */ }
            TelemetryEvent::SequenceMatched { confidence, snr, .. } => { /* rhythm classified */ }
            _ => {}
        }
    }
}
let output = brain.step_with_telemetry(freq, &mut telemetry);
```

### Batch processing (for offline analysis)
```rust
let outputs = brain.step_batch(&ecg_frequencies);
```

### STDP for adaptive threshold (optional)
```rust
brain.enable_stdp(StdpConfig::default().with_learning_rate(0.005));
brain.enable_homeostasis(HomeostasisConfig::default().with_target(0.3));
```

---

## 3. Python Binding API

```python
from cricket_brain import BrainConfig, Brain

config = BrainConfig()
config.min_freq = 3000.0
config.max_freq = 5000.0
config.adaptive_sensitivity = True
config.privacy_mode = True

brain = Brain(config)
output = brain.step(4500.0)          # Single step
outputs = brain.step_batch([4500.0] * 100)  # Batch
brain.reset()
print(brain.time_step())
```

---

## 4. C FFI API

```c
#include "cricket_brain.h"

BrainHandle *h = NULL;
int32_t err = brain_new(&h, 5, 3000.0, 5000.0);  // 5 neurons, 3-5 kHz

float output;
err = brain_step(h, 4500.0, &output);  // QRS frequency

BrainStatus status;
brain_get_status(h, &status);
printf("Step: %lu, Output: %f\n", status.time_step, status.last_output);

brain_free(h);
```

---

## 5. Dataset: MIT-BIH Arrhythmia Database

| Field | Value |
|-------|-------|
| Source | PhysioNet |
| URL | https://physionet.org/content/mitdb/1.0.0/ |
| License | Open Data Commons Attribution v1.0 |
| Records | 48 × 30 min two-channel ambulatory ECG |
| Sampling | 360 Hz, 11-bit, 10 mV range |
| Annotations | ~110,000 beat labels by 2+ cardiologists |
| Beat Types | N (Normal), S (Supraventricular), V (Ventricular), F (Fusion), Q (Unknown) |
| Citation | Moody & Mark (2001), Goldberger et al. (2000) |

### Preprocessing Strategy
1. Read .dat/.hea/.atr files with `wfdb` Python library
2. Extract R-R intervals from annotation timestamps
3. Convert R-R intervals to instantaneous frequencies: `freq = 1000.0 / rr_ms`
4. Map to CricketBrain input range: tune eigenfrequency to normal sinus BPM frequency
5. Store as CSV: `timestamp_ms, rr_interval_ms, beat_type, mapped_freq`

### Key Challenge
MIT-BIH samples at 360 Hz (raw amplitude), but CricketBrain expects frequency input.
We must extract temporal features (R-peak timing) first, then encode as frequency patterns.

---

## 6. Ten-Run Plan

### Run 1: Scaffold
**Deliverables:**
- `Cargo.toml` (standalone, depends on cricket-brain via path)
- `src/lib.rs` (empty module declarations)
- `src/main.rs` (hello world that creates a CricketBrain)
- `README.md` (generated from template with metrics)
- Verify: `cargo build` succeeds

### Run 2: Data Pipeline
**Deliverables:**
- `python/download_mitbih.py` — downloads MIT-BIH via wfdb
- `python/preprocess.py` — extracts R-R intervals, saves CSV
- `data/processed/sample_record.csv` — one processed record as example
- Verify: Python scripts run, CSV has correct columns

### Run 3: Core Detector
**Deliverables:**
- `src/ecg_signal.rs` — reads preprocessed CSV, maps to frequencies
- `src/detector.rs` — `CardiacDetector` struct wrapping CricketBrain
- `src/lib.rs` — exports detector
- `src/main.rs` — runs detector on sample record, prints classification
- Verify: `cargo run` classifies beats

### Run 4: Benchmark Suite
**Deliverables:**
- `benchmarks/cardiac_sdt.rs` — d' and AUC on MIT-BIH test split
- `benchmarks/cardiac_latency.rs` — first-spike latency per beat type
- `benchmarks/cardiac_memory.rs` — RAM footprint measurement
- Results written to stdout in structured format
- Verify: all benchmarks run, numbers are plausible

### Run 5: Python Analysis
**Deliverables:**
- `python/evaluate.py` — runs detector via Python binding, computes confusion matrix
- `python/plot_results.py` — generates ROC curve, latency histogram, confusion matrix plots
- `docs/results.md` — benchmark results with plots referenced
- Verify: plots generate, ROC curve looks reasonable

### Run 6: Stress Test
**Deliverables:**
- `benchmarks/cardiac_stress.rs` — adversarial conditions:
  - Noisy ECG (motion artifacts, baseline wander)
  - Unusual heart rates (30-250 BPM)
  - Edge cases (PVC, PAC, atrial flutter)
- `docs/limitations.md` — honest documentation of failure modes
- Verify: knows where it breaks, documents honestly

### Run 7: Website Demo
**Deliverables:**
- `website/index.html` — interactive ECG demo page
  - Synthetic ECG waveform visualization
  - Real-time beat classification display
  - Metric cards from metrics.json
- Uses same CSS as main site
- Verify: opens in browser, looks professional

### Run 8: Documentation
**Deliverables:**
- Complete `README.md` with:
  - Architecture diagram
  - API examples (Rust, Python, C)
  - Benchmark results table
  - Medical disclaimer (prominent)
  - Setup instructions
- `docs/api.md` — detailed API reference
- Verify: README renders correctly on GitHub

### Run 9: CI Integration
**Deliverables:**
- CI job in `.github/workflows/ci.yml` (or separate workflow)
  - `cargo build --manifest-path use_cases/01_cardiac_arrhythmia/Cargo.toml`
  - `cargo test --manifest-path use_cases/01_cardiac_arrhythmia/Cargo.toml`
- Verify: CI passes

### Run 10: Metrics Finalization
**Deliverables:**
- Update `metrics.json` with real measured values:
  - `accuracy_pct` (from SDT benchmark)
  - `latency_ms` (from latency benchmark)
  - `false_positive_rate` (from stress test)
- Run `inject_metrics.py` to propagate everywhere
- Update this CLAUDE.md: mark all runs as done
- Verify: `inject_metrics.py --dry-run` shows 0 pending changes

---

## 7. Metrics to Write After Run 10

```json
{
  "01_cardiac_arrhythmia": {
    "accuracy_pct": <measured>,
    "latency_ms": <measured>,
    "false_positive_rate": <measured>,
    "d_prime": <measured>,
    "auc": <measured>,
    "ram_bytes": <measured via memory_usage_bytes()>,
    "beats_analyzed": <total from MIT-BIH>,
    "beat_types_tested": ["N", "S", "V", "F"]
  }
}
```

---

## 8. Run Status

| Run | Status | Date | Notes |
|-----|--------|------|-------|
| 0 | DONE | 2026-04-10 | Scaffold directories, SOURCES.md, CLAUDE.md |
| 1 | DONE | 2026-04-10 | Cargo.toml, src/, README, 7/7 tests pass, BPM correct |
| 2 | DONE | 2026-04-10 | Data pipeline: Python download/preprocess, CSV I/O, 9/9 tests |
| 3 | DONE | 2026-04-10 | CSV integration, confusion matrix, 11/11 tests, 92.5% accuracy |
| 4 | DONE | 2026-04-10 | SDT d'=6.18, Latency 0.126µs/step, RAM 928B=match, Criterion bench |
| 5 | DONE | 2026-04-10 | evaluate.py (F1=0.962), 3 plots, docs/results.md |
| 6 | DONE | 2026-04-10 | 5 adversarial tests, noise fails >10%, boundary ±1BPM works |
| 7 | DONE | 2026-04-10 | website/pages/cardiac.html, nav+footer+SPA linked |
| 8 | DONE | 2026-04-10 | Full README with real results, docs/api.md reference |
| 9 | DONE | 2026-04-10 | CI workflow uc01-cardiac.yml, all steps verified locally |
| 10 | DONE | 2026-04-10 | metrics.json updated with all measured values, inject verified |

---

## 9. Next Prompt

--- NEXT PROMPT START ---
Lies use_cases/01_cardiac_arrhythmia/CLAUDE.md und fuehre Run 3 aus.

Run 7 Deliverables — Website Demo:

1. Erstelle use_cases/01_cardiac_arrhythmia/benchmarks/cardiac_stress.rs:
   - Adversarial-Bedingungen fuer den Cardiac Detector:
   a) Noisy ECG: Zufaellige Frequenz-Spikes waehrend QRS (Bewegungsartefakte)
   b) Extreme Raten: 30, 40, 50, 60, 80, 100, 120, 150, 200, 250 BPM
   c) Wechselnde Rhythmen: schneller Wechsel Normal↔Tachy alle 3 Beats
   d) Near-boundary: 59 BPM (knapp Brady), 61 BPM (knapp Normal), 99/101 BPM
   e) Irregular: Zufaellige RR-Intervalle (300-1200ms)
   - Fuer jeden Test: TPR, FPR, Accuracy, ehrliche Grenzen

2. Erstelle use_cases/01_cardiac_arrhythmia/docs/limitations.md:
   - Zusammenfassung aller bekannten Schwaechen
   - Wo genau bricht die Detektion zusammen?
   - Vergleich: was kann CricketBrain NICHT vs. Deep-Learning-ECG-Systeme

3. Verifiziere:
   - cargo run --release --example cardiac_stress
   - Ergebnisse zeigen ehrliche Grenzen

4. Update CLAUDE.md: Run 6 = DONE, NEXT PROMPT fuer Run 7

REGELN:
- Aendere NICHTS ausserhalb von use_cases/
- Commit und push am Ende
--- NEXT PROMPT END ---
