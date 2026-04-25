# UC01 Cardiac — Benchmark Methodology

This document explains *how* the UC01 cardiac benchmark suite scores
the detector. It is the reviewer-facing companion to
[BENCHMARK_ROADMAP.md](../BENCHMARK_ROADMAP.md) and
[docs/results.md](results.md).

## 1. Ground truth

**Synthetic ground truth comes from the generator, never from the
detector.** Each `LabeledSegment` produced by
[`crate::synthetic::generate`](../src/synthetic.rs) carries an
explicit `RhythmClass` field that was decided *before* the detector
ever runs. When the detector emits a classification at step `i`, the
benchmark looks up the segment containing step `i` and pairs the
detector's prediction with that segment's class label.

The legacy `ConfusionMatrix::from_predictions` in `src/detector.rs`
derives ground truth from the detector's own BPM estimate. That is
**circular** — predictions agree with their own BPM-derived label by
construction. The new
[`crate::evaluate::run_and_score`](../src/evaluate.rs) replaces this.
The legacy function is kept only for backward compatibility with the
demo binary and is not used by any v0.2 result file.

## 2. Class layout

Four ground-truth classes:

| Index | Class | Synthetic RR range |
|------:|-------|--------------------|
| 0 | NormalSinus | 700 – 1000 ms (60 – 86 BPM) |
| 1 | Tachycardia | 330 – 500 ms (120 – 182 BPM) |
| 2 | Bradycardia | 1200 – 1800 ms (33 – 50 BPM) |
| 3 | Irregular | random RR in 300 – 1200 ms |

Block layout of one synthetic recording:
```
[ Normal block ][ Tachy block ][ Brady block ][ Irregular block ]
```
Block size is `beats_per_class` beats; segments are contiguous and
cover the whole stream.

## 3. Warmup convention

Every interval-based classifier needs ≥ 2 RR intervals before its
output is stable. The benchmarks always report **two** confusion
matrices:

- `aggregate_full` — every emission, including transitions
- `aggregate_after_warmup` — first `warmup` emissions per segment
  dropped (default `warmup = 2`)

This makes transition zones visible without hiding them, and prevents
the "Irregular" class from being dominated by warmup artifacts.

## 4. Reject-aware metrics

`coverage_accuracy_curve(samples, thresholds)` sweeps a confidence
threshold over the detector's emission stream. For each threshold:

- `coverage` = fraction of emissions for which `confidence ≥ threshold`
- `covered_accuracy` = correct / covered (accuracy when committing)
- `forced_accuracy` = correct / total (treats reject as wrong, lower bound)

This curve is the right way to think about embedded triage: the
system is allowed to abstain. `cardiac_reject` writes the full curve
to `results/cardiac_reject_curve.csv`.

## 5. Stress sweeps

`benchmarks/cardiac_stress_sweep.rs` parameterises seven stress
dimensions (see roadmap § 7) and writes per-dimension CSVs. Sweep
ranges are chosen to cover the regime where the detector goes from
"works" to "broken", *not* to flatter the detector. Every CSV row
includes the full set of metrics, not just accuracy, so reviewers
can plot per-class recall directly.

## 6. Baselines

Two non-neuromorphic baselines run on the same labelled stream:

- `ThresholdBurstBaseline` — band-gate + RR-window with the same
  rate-regime classification logic as the detector. Used to isolate
  the contribution of CricketBrain's coincidence gate.
- `FrequencyRuleBaseline` — 1-second window QRS-burst counter.
  Deliberately primitive; if even this matches CricketBrain in a
  scenario, the scenario does not need a neuromorphic core.

A claim of "CricketBrain works" must beat at least the
threshold-burst baseline in the relevant scenario. `cardiac_baselines`
makes this comparison automatic.

## 7. Determinism

The synthetic generator is built on a SplitMix64-style RNG. Every
random draw is derived from a single `u64` seed. Two runs with the
same `--seed` and the same `--beats-per-class` produce byte-identical
streams (this is unit-tested in
`synthetic::tests::deterministic_for_same_seed`).

Detector state is reset between scenarios. CricketBrain's own
internal seed is independent and is the one already chosen by the
detector (`with_seed(42)`).

## 8. Result file metadata

Every JSON / CSV result file written by the v0.2 benches includes:

- `generated_at` (ISO-8601 UTC)
- `git_commit` (best-effort `git rev-parse --short HEAD`)
- `command` (the CLI that produced the file)
- `dataset_type` (`synthetic` or `csv`)
- `dataset_name`
- `synthetic_generator_version` (`0.2.0`)
- `seed`
- `classes`
- `window_size_ms`
- `sample_rate_hz`
- `limitations`

This is what makes a result file self-contained: a reviewer who
finds a stray `cardiac_synthetic_summary.json` on disk can tell
exactly which commit produced it, with which seed, on which
generator version.

## 9. What is *not* measured here

- **Real ECG morphology.** No P-wave morphology, no ST analysis, no
  QT, no QRS width, no axis. The frequency input is a 1-ms-resolution
  carrier-aligned representation, not a clinical waveform.
- **Patient identity.** No patient-level metadata, no inter-patient
  splits — the synthetic generator has no concept of "patient".
- **Power consumption.** Latency is measured in µs/step on the host
  CPU; embedded power is *not* directly measured. SynOPS-style
  estimation is on the roadmap (v0.6).
- **Clinical outcomes.** No survival, no false-discharge rate, no
  reader study. UC01 is *not* a clinical study and never claims to be.

## 10. Reproducing a result file

```bash
# from the repo root
cargo run --release --quiet \
    --manifest-path use_cases/01_cardiac_arrhythmia/Cargo.toml \
    --example cardiac_eval -- \
    --seed 42 --beats-per-class 30 --write
```

Outputs land in `use_cases/01_cardiac_arrhythmia/results/`. The set
of file names produced by each example is documented in the example's
top-of-file comment.

The single-command "run every benchmark and regenerate every artifact"
target is on the v1.0 roadmap.
