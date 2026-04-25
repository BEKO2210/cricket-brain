# Data Sources — Power-Grid Harmonic & Stability Triage

## Primary Dataset

**EPFL Smart Grid Dataset (Distribution Test Network)**

| Field | Value |
|-------|-------|
| Source | École Polytechnique Fédérale de Lausanne, DESL Lab |
| URL | https://www.epfl.ch/labs/desl-pwrs/smart-grid/ |
| License | CC BY 4.0 |
| Equipment | OpenPMU + EPFL synchrophasor measurement units |
| Rate | 50 frame/s synchrophasor + 50 kHz aux waveform capture |
| Typical signals | Voltage (V), current (A), frequency (Hz) |
| Coverage | Years of recorded distribution-network events |
| Format | CSV / HDF5 |

## Target Frequency Targets (50 Hz EU System)

| Channel | Frequency | Real-world meaning |
|---------|-----------|--------------------|
| FUND | 50 Hz | Healthy fundamental |
| H2 | 100 Hz | DC offset, transformer in-rush, half-wave rectified loads |
| H3 | 150 Hz | **Most common power-quality issue** — non-linear loads (rectifiers, VFDs, SMPS, LED ballasts, arc furnaces) |
| H4 | 200 Hz | Fast switching artefacts, EMI from RF power-electronics |

These four frequencies are exact integer multiples of 50 Hz so the
`TokenVocabulary::new(&[...], 50.0, 200.0)` distribution lands precisely
on each tuned channel.

## Reference Standards

| Standard | Topic |
|----------|-------|
| IEEE 519 | Voltage and current distortion limits in electric power systems |
| IEC 61000-4-30 | Power-quality measurement methods (Class A) |
| IEC 61000-4-7 | Harmonics and inter-harmonics measurement guide |

## Download Instructions

```bash
# EPFL data is published under CC BY 4.0 with no registration required.
# Visit the DESL portal, download a PMU CSV segment, and run the
# preprocessing pipeline:

python python/preprocess.py --pmu data/raw/pmu_segment.csv --label Nominal
```

## Citation

École Polytechnique Fédérale de Lausanne, DESL Lab, "EPFL Smart Grid
Distribution Test Network and Synchrophasor Dataset,"
https://www.epfl.ch/labs/desl-pwrs/smart-grid/

## Usage Notes

- **CC BY 4.0** — attribution required, commercial use permitted.
- For 60 Hz systems (US, Canada, parts of South America, Japan partial)
  remap channels to 60 / 120 / 180 / 240 Hz by passing those values to
  `TokenVocabulary::new` in `src/detector.rs`.
- Off-nominal **fundamental drift** (49.5-50.5 Hz) is detected as
  `Nominal` by this triage detector. For ±0.1 Hz precision use a
  dedicated PMU and feed the precise frequency reading downstream.
