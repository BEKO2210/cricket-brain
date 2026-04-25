# UC01 Cardiac — Benchmark Roadmap

> **Position:** CricketBrain UC01 is a deterministic, KB-class ECG
> rhythm-pattern triage core for **research and embedded
> pre-screening**. It detects candidate rhythm/event patterns,
> exposes uncertainty instead of forcing decisions, and is **not** a
> diagnostic medical device.
>
> **This roadmap is honest, not aspirational.** Every claim labelled
> "proven" is reproducible by running a single `cargo run --release
> --example …` command in this directory. Every claim labelled
> "planned" has no data behind it yet.

This file replaces the marketing-style "world-class" framing. It
defines what would actually be required for the UC01 synthetic
benchmark suite to qualify as **best-in-class for ultra-low-memory
deterministic ECG rhythm-pattern triage**, and tracks the gap
between current state and that bar.

---

## 1. Current benchmark status (as of v0.2 hardening)

| Area | Status |
|---|---|
| Truth-based confusion matrix (4 classes incl. Irregular) | Proven |
| Per-class precision / recall / F1 / specificity | Proven |
| Macro-F1, weighted-F1, balanced accuracy | Proven |
| Reject-aware coverage / accuracy curve | Proven |
| Deterministic seeded synthetic generator (SplitMix64) | Proven |
| Stress sweeps: noise, baseline wander, amp jitter, HRV, morph jitter, missing QRS, motion burst | Proven |
| Simple baselines (threshold-burst rule, frequency rule) | Proven |
| Structured JSON / CSV output with metadata, git commit, seed | Proven |
| Auto-captured failure cases (markdown table) | Proven |
| Latency / RAM benchmarks | Proven (legacy `cardiac_latency`, `cardiac_memory`) |
| SDT d′ benchmark with Wilson CI | Proven (legacy `cardiac_sdt`) |
| MIT-BIH loader & CSV pipeline (skeleton) | Skeleton only |
| Real-data confusion matrix on MIT-BIH | Planned |
| Patient-level data splits (no inter-patient leakage) | Planned |
| Calibration / reliability diagrams | Planned |
| Cross-platform reproducibility hash | Planned |
| Public reviewer artifact bundle | Planned |

---

## 2. What is already strong

- **Deterministic core.** Same seed → identical stream → identical
  emissions → identical metrics. There is no source of nondeterminism
  in the synthetic pipeline.
- **No serde / no random crates.** Result writers are 100% in-tree;
  any reviewer can audit the whole pipeline without a third-party
  serialiser.
- **Stress dimensions are first-class.** Every stress dimension is a
  `SyntheticConfig` knob, not a one-off code path inside a
  benchmark binary. This is what allows future sweeps to be added in
  five lines.
- **Reject-aware metrics.** `coverage_accuracy_curve` exposes the
  triage operating point directly — at confidence ≥ 0.9 the detector
  rejects ~39 % of decisions but is 100 % accurate on the remainder.
  That curve is the right way to think about embedded triage.
- **Honest baselines.** The threshold-burst baseline already matches
  CricketBrain on clean synthetic data and *beats* it under 2 % spike
  noise. The benchmark suite makes that fact visible instead of
  hiding it.
- **Footprint stays at 928 bytes.** Adding metrics, stress and
  baseline modules did not enlarge the runtime detector at all —
  everything new lives in `benchmarks/` or in compile-time test code.

## 3. What is weak

- **Synthetic only.** All numbers in `results/` are produced by
  `synthetic::generate`. No real-patient data has been ingested.
- **Brittleness to morphology jitter.** A 5 % per-cycle jitter on
  P/QRS/T frequencies and durations is enough to drive macro-F1
  below 0.10. The detector's coincidence gate is tightly tuned to
  the carrier; even modest morphology drift breaks it. This is the
  single biggest finding of the v0.2 audit.
- **CricketBrain ≈ trivial baseline on clean data.** With QRS
  perfectly aligned to 4500 Hz, a band-gate + RR-window rule is
  equivalent to the full neuromorphic detector. The neuromorphic
  contribution is currently invisible in synthetic numbers.
- **Confidence is heuristic.** `confidence = 0.4·data + 0.6·(1−CV)`
  is a sensible but uncalibrated score. There is no reliability
  diagram, no Brier score, no calibration on held-out data.
- **No patient-level splits.** Even the synthetic pipeline doesn't
  enforce the kind of split that real-data evaluation will require.
- **No cross-platform hash.** Numbers may shift if anyone changes
  the floating-point order of operations in CricketBrain core.

## 4. What would make it world-class

To honestly qualify as the **strongest open benchmark suite for
KB-class deterministic ECG rhythm-pattern triage**, the suite would
need every item below — and would need each item *visible from the
result files alone*:

1. **Real MIT-BIH inter-patient validation** — patient-disjoint
   train/test, AAMI-compatible class mapping, and a confusion matrix
   that does not silently lump S/V/F/Q into Irregular.
2. **Calibration plot.** A reliability diagram of `confidence` vs
   actual accuracy, plus Brier score and ECE.
3. **Deterministic reproducibility hash.** Ship a `result_hash.txt`
   computed over `cardiac_synthetic_summary.json` for a fixed seed
   — any reviewer can re-run and bit-compare.
4. **Cross-seed robustness.** Repeat each benchmark over 10 seeds
   and report mean ± std of every metric — single-seed numbers don't
   survive review.
5. **Public reviewer bundle.** A single `make review` (or
   `cargo xtask review`) target that runs every benchmark, regenerates
   every result file, and emits a single tarball.
6. **Architectural ablation.** Disable the coincidence gate, the
   adaptive sensitivity, the privacy mode, the preprocessor — one at
   a time — and quantify the contribution of each. If the
   coincidence gate adds zero macro-F1 over the threshold baseline
   in scenario X, that needs to be on the table.
7. **Power / energy proxy.** A SynOPS-style estimate of compute
   energy per decision, derived from active-neuron events, validated
   against measured wall-clock latency.
8. **Adversarial morphology bank.** A handcrafted bank of cases
   that classical thresholding handles but the current detector
   misclassifies, and vice versa.
9. **Per-channel tuning ablation.** What if the carrier is moved off
   QRS frequency? How wide is the basin of attraction of the current
   tuning? (Currently brittle, see § 3 amp/morph findings.)

## 5. Required datasets

| Dataset | Status | Use |
|---|---|---|
| Synthetic generator (`src/synthetic.rs`) | Available | Hardening, sweeps, regressions |
| MIT-BIH Arrhythmia (PhysioNet, ODC-By v1.0) | Loader skeleton, not validated | Inter-patient triage validation |
| AHA / Long-Term ST DB | Planned | Out-of-distribution morphology stress |
| BIDMC PPG-ECG (CC BY 4.0) | Planned | Wearable-style noise stress |
| Apple Watch / Polar H10 traces | Planned (consent permitting) | Real-world artifacts |

Real-data ingestion will go through the existing
`python/download_mitbih.py` + `python/preprocess.py` path. **No real
data is committed to this repo.**

## 6. Required metrics (target inventory)

- Accuracy (overall, after-warmup)
- Per-class precision / recall / F1 / specificity
- Macro-F1 / weighted-F1 / balanced accuracy ✓
- Confusion matrix (4-class, with Irregular) ✓
- Reject-aware coverage / accuracy curve ✓
- d′ / AUC with Wilson CI (legacy)
- Brier score / Expected Calibration Error
- Reliability diagram bins
- False-negative rate on candidate "abnormal" classes (sensitivity-weighted)
- Time-to-first-classification per regime ✓ (legacy)
- Cost-sensitive metric (e.g. `cost = 5·FN_abnormal + 1·FP`) — planned

## 7. Required stress tests

- Random in-band noise spikes ✓
- Slow-drift baseline wander ✓
- Per-beat QRS frequency / amplitude jitter ✓
- HRV (RR jitter) ✓
- Morphology jitter (P/QRS/T freq + duration) ✓
- Missing-QRS / weak-peak ✓
- Motion-artifact bursts ✓
- Polarity flip / lead inversion — planned
- Sample-rate variation — planned (currently fixed at 1 ms)
- Long recordings (≥ 1 hour synthetic) — planned

## 8. Required baselines

- Threshold-burst rule ✓
- 1-second frequency-rule window ✓
- Pan-Tompkins reference (open implementation) — planned
- Tiny CNN (8-bit) — planned, comparison only, **never** vendored as
  a CricketBrain implementation

## 9. Required reproducibility guarantees

- Fixed seeds documented in CLI flags ✓
- JSON / CSV output with `generated_at`, `git_commit`, `command`,
  `dataset_type`, `dataset_name`, `synthetic_generator_version`,
  `seed`, `classes`, `window_size_ms`, `sample_rate_hz`,
  `limitations` ✓
- One-shot regeneration command — planned (`cargo run --example
  cardiac_review` or `make review`)
- Bit-stable cross-platform hashes — planned
- Documented compiler / Rust version (CI captures this) ✓

## 10. Required review artifacts

- `results/*.json` and `results/*.csv` (committed only when
  refreshed by a real run) ✓
- `results/cardiac_failure_cases.md` (auto) ✓
- `BENCHMARK_ROADMAP.md` (this file) ✓
- `docs/methodology.md` — design decisions, ground-truth definition,
  warmup convention, why circular truth was rejected — planned, this
  PR adds the first version
- `docs/limitations.md` ✓ (already strong)
- `docs/results.md` — synced with real numbers and clearly labelled
  "synthetic-window accuracy" — being updated

## 11. Milestones

| Milestone | Definition of done | Status |
|---|---|---|
| **v0.1** synthetic benchmark hardening | Truth-based metrics + structured outputs + stress sweeps + baselines | **Done** (this PR) |
| **v0.2** reject-aware benchmark | Coverage / accuracy curve + reliability artifact | Curve done; reliability diagram pending |
| **v0.3** MIT-BIH loader + first real-data run | Real CSV → real confusion matrix (no fake numbers) | Loader skeleton ready; data ingestion pending |
| **v0.4** real-data confusion matrix + failure cases | Inter-patient split, MIT-BIH `results/`, automated failure md | Pending |
| **v0.5** baseline comparison on real data | Pan-Tompkins reference + Tiny CNN reference | Pending |
| **v0.6** ablations | Component contribution table (gate / AGC / preproc / privacy) | Pending |
| **v0.7** cross-seed robustness | Mean ± std over 10 seeds for every metric | Pending |
| **v0.8** calibration | Reliability diagram + Brier + ECE | Pending |
| **v1.0** reproducible benchmark report | One command rebuilds every artifact; bit-stable hash; reviewer bundle | Pending |

## 12. What this roadmap explicitly will *not* try to do

- Replace KardiaMobile, Apple Watch AFib, Holter analysers, or any
  certified clinical device.
- Claim 100 % accuracy on anything.
- Publish "MIT-BIH validated" numbers before MIT-BIH has actually
  been ingested.
- Ship hardcoded benchmark scores in any form other than
  human-readable static doc tables clearly labelled "example".
- Inflate medical claims to drive marketing.

The roadmap measures success by **how easy it is for a sceptical
external reviewer to disprove a claim**. Every milestone above
exists because, today, that reviewer would have a reasonable
objection that the suite cannot answer.
