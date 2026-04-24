# Data Sources — Marine Acoustic Monitoring

## Primary Dataset

**MBARI MARS Cabled Observatory Hydrophone**

| Field | Value |
|-------|-------|
| Name | Monterey Accelerated Research System (MARS) hydrophone |
| URL | https://www.mbari.org/technology/mars/ |
| Data Portal | https://docs.mbari.org/pacific-sound/ |
| License | CC BY 4.0 |
| Location | Monterey Bay, California, 891 m depth |
| Sampling | 256,000 Hz (broadband), decimated to 2 kHz for baleen whale work |
| Bit depth | 24-bit |
| Calibration | -177.9 dB re 1 V/µPa (nominal) |
| Coverage | Continuous recording since 2015 |
| Format | FLAC (lossless) archived per 24 hours |

## Target Acoustic Signatures

| Abbreviation | Frequency | Source |
|--------------|-----------|--------|
| FIN | 20 Hz | Fin whale (*Balaenoptera physalus*) 20-Hz stereotyped pulse |
| BLUE | 80 Hz | Blue whale (*Balaenoptera musculus*) NE-Pacific A-call tonal |
| SHIP | 140 Hz | Cargo-ship radiated noise peak (propeller cavitation, 10-14 knots) |
| HUMP | 200 Hz | Humpback whale (*Megaptera novaeangliae*) song mid-band unit |

These frequencies sit inside the 10-500 Hz baleen whale band routinely
monitored by MBARI's automated species-identification pipeline and ESONS
(European Seas Observatory Network) PAMGuard classifiers.

## Download Instructions

```bash
# MARS hydrophone data is hosted on AWS S3 under the Open Data Sponsorship
# programme. Install the AWS CLI and fetch a single FLAC file:

aws s3 cp --no-sign-request \
    s3://pacific-sound-2khz/2023/01/MARS-20230115T000000Z.flac \
    data/raw/

# Decode to WAV and run the preprocessing pipeline:
ffmpeg -i data/raw/MARS-20230115T000000Z.flac data/raw/mars_sample.wav
python python/preprocess.py --wav data/raw/mars_sample.wav
```

## Citation

John P. Ryan, Danelle E. Cline, Kelly J. Benoit-Bird et al., "Oceanic
giants dance to atmospheric rhythms," *Geophysical Research Letters*, 2019.

MBARI, "Monterey Accelerated Research System (MARS) underwater observatory,"
https://www.mbari.org/technology/mars/

## Usage Notes

- **CC BY 4.0** — attribution required, commercial use permitted.
- MBARI hosts >8 years of continuous data (~2 PB) under the AWS Open Data
  programme; no registration needed.
- The MARS cabled observatory is widely used for cetacean PAM (Passive
  Acoustic Monitoring) research and marine traffic studies.
- Fin whale 20-Hz pulses are the most stereotyped cetacean vocalisation
  on Earth — a perfect benchmark target for frequency-resonance detectors.
