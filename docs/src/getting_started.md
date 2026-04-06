# Getting Started

Cricket-Brain is a biomorphic inference engine for temporal pattern detection.

## Install & Run

```bash
cargo run
cargo test
```

## Research Data (Headless)

```bash
cargo run --release --example research_gen -- --headless --output target/research --seed 1337
```

This produces CSV/JSON artifacts for SNR sweeps and ROC analysis.

## Cross-language wrappers

- Python (`crates/python`): build with `maturin develop`
- TypeScript/WASM (`crates/wasm`): build with `wasm-pack build --target web`
