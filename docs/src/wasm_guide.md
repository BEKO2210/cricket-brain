# WASM Guide

Build the browser package with `wasm-pack` from `crates/wasm`:

```bash
cd crates/wasm
wasm-pack build --target web --out-dir pkg
```

This generates JavaScript glue + TypeScript definitions (`.d.ts`) for `Brain`.

Demo template:

- `examples/wasm_demo/index.html`
- `examples/wasm_demo/main.ts`

See full notes in [`README_WASM.md`](../../README_WASM.md).
