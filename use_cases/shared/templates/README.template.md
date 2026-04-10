# CricketBrain Application: [USE CASE NAME]

> **Status:** In Development | **CricketBrain v{{global.version}}** | **License:** {{global.license}}

---

## Overview

[Describe the problem this application solves and why CricketBrain is the right tool.]

**Market Size:** ${{uc.market_size_usd_bn}}B | **Key Advantage:** {{uc.key_advantage}}

## CricketBrain Properties

| Property | Value |
|----------|-------|
| RAM Footprint | {{global.ram_bytes}} bytes |
| Latency | {{global.latency_us}} µs/step |
| Throughput | {{global.throughput_steps_per_sec}} steps/sec |
| Neurons | {{global.neurons}} |
| Synapses | {{global.synapses}} |
| Checksum | {{global.checksum}} |

## Dataset

| Field | Value |
|-------|-------|
| Primary Dataset | {{uc.dataset_primary}} |
| License | {{uc.dataset_license}} |
| Signal Rate | {{uc.signal_hz}} Hz |

## Results

| Metric | Value |
|--------|-------|
| Accuracy | {{uc.accuracy_pct}} |
| Latency | {{uc.latency_ms}} ms |
| False Positive Rate | {{uc.false_positive_rate}} |

> Results marked **TBD** will be filled in after benchmarking against the dataset.

## Quick Start

```bash
cd use_cases/[USE_CASE_DIR]
cargo build
cargo run
cargo test
```

## Project Structure

```
├── data/
│   ├── raw/           # Original dataset files (not committed)
│   ├── processed/     # Preprocessed signals
│   └── SOURCES.md     # Dataset provenance and download instructions
├── src/               # Rust source code
├── tests/             # Integration tests
├── benchmarks/        # Performance benchmarks
├── python/            # Python analysis scripts
├── docs/              # Application-specific documentation
└── website/           # Web demo page
```

## Medical / Safety Disclaimer

> **This application is NOT a certified medical device.** It is a research prototype
> for educational and experimental purposes only. Do not use for clinical diagnosis,
> patient monitoring, or any safety-critical decision-making without appropriate
> regulatory approval (FDA, CE, etc.).

## References

- [CricketBrain Research Whitepaper](../../RESEARCH_WHITEPAPER.md)
- [USE_CASES.md](../../USE_CASES.md)
- [Contributing Guide](../../CONTRIBUTING.md)

## Metrics Source

All metrics in this document are injected from
[`use_cases/shared/metrics.json`](../shared/metrics.json).
Run `python use_cases/shared/scripts/inject_metrics.py` to update.
