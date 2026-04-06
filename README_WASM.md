# WASM Build Notes (`wasm32-unknown-unknown`)

`cricket-brain` core logic is compatible with WebAssembly builds.

## 1) Install target

```bash
rustup target add wasm32-unknown-unknown
```

## 2) Build library for web runtimes

```bash
cargo build --target wasm32-unknown-unknown --no-default-features
```

Or with default features:

```bash
cargo build --target wasm32-unknown-unknown
```

## 3) Notes

- CLI (`examples/cricket_cli.rs`) is behind the `cli` feature and uses OS I/O (`std::fs`, stdout/file streaming), so keep it disabled for pure wasm library builds.
- Core crate keeps `#![deny(unsafe_code)]`.
- FFI crate is intended for native C ABI use (`cdylib`/`staticlib`), not browser JS bindings.
