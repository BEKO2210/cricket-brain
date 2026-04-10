# Data Sources — Predictive Bearing Maintenance

## Primary Dataset

**CWRU Bearing Data Center**

| Field | Value |
|-------|-------|
| Name | Case Western Reserve University Bearing Data Center |
| URL | https://engineering.case.edu/bearingdatacenter |
| License | Public Domain |
| Bearing | SKF 6205-2RS deep groove ball bearing |
| Sampling | 12,000 Hz (drive end) and 48,000 Hz |
| Motor | 2 HP Reliance Electric, 1797 RPM |
| Faults | Inner race, outer race, ball defects |
| Fault sizes | 0.007", 0.014", 0.021" diameter EDM |
| Loads | 0, 1, 2, 3 HP |
| Format | .mat (MATLAB) files |

## Characteristic Defect Frequencies (SKF 6205-2RS)

| Abbreviation | Frequency | Defect |
|--------------|-----------|--------|
| BPFO | 107.36 Hz | Ball Pass Frequency Outer race |
| BPFI | 162.19 Hz | Ball Pass Frequency Inner race |
| BSF | 69.04 Hz | Ball Spin Frequency |
| FTF | 14.83 Hz | Fundamental Train Frequency |

Calculated for 1797 RPM shaft speed, 9 balls, 0.3126" ball diameter,
1.122" pitch diameter, 0° contact angle.

## Download Instructions

```bash
# The CWRU data must be downloaded manually from the website.
# Visit https://engineering.case.edu/bearingdatacenter/download-data-file
# Download the .mat files for the desired conditions.
# Place in data/raw/
```

## Citation

K.A. Loparo, "Bearings Data Center," Case Western Reserve University,
https://engineering.case.edu/bearingdatacenter

## Usage Notes

- **Public Domain** — no restrictions on use
- The dataset is the most widely cited benchmark in bearing fault diagnosis
- Over 500 papers reference this dataset
- Data includes normal baseline and 12 fault conditions
