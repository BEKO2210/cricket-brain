# Data Sources — Cardiac Arrhythmia Pre-Screening

## Primary Dataset

**MIT-BIH Arrhythmia Database**

| Field | Value |
|-------|-------|
| Name | MIT-BIH Arrhythmia Database |
| URL | https://physionet.org/content/mitdb/1.0.0/ |
| License | Open Data Commons Attribution License v1.0 |
| Citation | Moody GB, Mark RG. The impact of the MIT-BIH Arrhythmia Database. IEEE Eng in Med and Biol 20(3):45-50, 2001 |
| Goldberger AL, et al. | PhysioBank, PhysioToolkit, and PhysioNet. Circulation 101(23):e215-e220, 2000 |
| Records | 48 half-hour excerpts of two-channel ambulatory ECG |
| Sampling Rate | 360 Hz |
| Resolution | 11-bit over 10 mV range |
| Annotations | ~110,000 beat annotations by cardiologists |

## Download Instructions

### Option 1: wget (recommended)

```bash
# From the project root:
cd use_cases/01_cardiac_arrhythmia/data/raw/

# Download all records (approx. 100 MB)
wget -r -N -c -np https://physionet.org/files/mitdb/1.0.0/
```

### Option 2: PhysioNet CLI

```bash
pip install wfdb
python -c "import wfdb; wfdb.dl_database('mitdb', 'use_cases/01_cardiac_arrhythmia/data/raw/mitdb')"
```

### Option 3: Manual

1. Visit https://physionet.org/content/mitdb/1.0.0/
2. Click "Files" tab
3. Download all `.dat`, `.hea`, and `.atr` files
4. Place in `data/raw/`

## File Formats

| Extension | Content |
|-----------|---------|
| `.dat` | Signal data (binary, 212 format) |
| `.hea` | Header (text, describes signal layout) |
| `.atr` | Annotations (binary, beat labels) |

## Usage Restrictions

- **License:** Open Data Commons Attribution License v1.0
- **Attribution required:** Must cite Goldberger et al. (2000) and Moody & Mark (2001)
- **Commercial use:** Permitted with attribution
- **Redistribution:** Permitted with attribution
- **No warranty:** Data provided as-is, no guarantees of accuracy

## Important Notes

1. **Do NOT commit raw data to git.** The `data/raw/` directory is for local use only.
   Add large data files to `.gitignore`.
2. **Patient privacy:** The MIT-BIH database is fully anonymized. No re-identification
   is possible from the published signals.
3. **Not for clinical use:** This dataset is for research and algorithm development only.
   Any clinical application requires independent validation with certified data.

## Preprocessing Pipeline

The raw 360 Hz signals will be preprocessed to extract:
1. R-R intervals (beat-to-beat timing)
2. QRS complex frequency signatures
3. Rhythm classification labels (N, S, V, F, Q per AAMI standard)

Preprocessed data will be stored in `data/processed/` as CSV or binary arrays.
