# C/C++ Integration

Use the FFI crate and generated header:

```bash
cargo build -p cricket-brain-ffi
```

Header path:

- `crates/ffi/include/cricket_brain.h`

Lifecycle:

1. `brain_new(...)`
2. `brain_step(...)`
3. `brain_get_status(...)`
4. `brain_free(...)`
