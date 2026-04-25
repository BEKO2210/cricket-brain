# Cardiac UC01 — auto-captured failure cases

Generated: 2026-04-25T09:56:19Z | seed: 42 | dataset: `uc01_synth_v0.2_seed42_bpc30`

Each row is one detector emission whose **prediction differed from the ground-truth segment label**. The detector reports a BPM estimate at emission time; the ground-truth column is taken from the labelled synthetic segment in which the emission occurred.

| step | truth | predicted | bpm | confidence |
|-----:|:------|:----------|----:|-----------:|
| 25672 | Tachy | Normal | 84 | 0.85 |
| 26029 | Tachy | Irregular | 89 | 0.81 |
| 26431 | Tachy | Irregular | 96 | 0.78 |
| 26837 | Tachy | Irregular | 109 | 0.80 |
| 27275 | Tachy | Irregular | 120 | 0.81 |
| 40509 | Brady | Irregular | 83 | 0.54 |
| 42051 | Brady | Irregular | 69 | 0.59 |
| 43270 | Brady | Irregular | 61 | 0.65 |
| 45035 | Brady | Irregular | 52 | 0.70 |
| 46274 | Brady | Irregular | 48 | 0.74 |
| 47936 | Brady | Irregular | 43 | 0.82 |
| 86125 | Irregular | Brady | 43 | 0.84 |
| 102303 | Irregular | Normal | 81 | 0.82 |
| 102883 | Irregular | Normal | 87 | 0.82 |
