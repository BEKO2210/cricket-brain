# Contributing to CricketBrain

Thank you for your interest in contributing to CricketBrain!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/cricket-brain.git`
3. Create a feature branch: `git checkout -b feat/my-feature`
4. Install Rust 1.75+: `rustup update stable`

## Development Workflow

```bash
# Run all tests
cargo test --workspace

# Check formatting
cargo fmt --all -- --check

# Run linter (must pass with zero warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Verify no_std core builds
cargo build -p cricket-brain-core --no-default-features

# Run benchmarks
cargo bench
```

## Code Standards

- **no_std in core:** The `crates/core` crate must remain `no_std` compatible.
  Never add `std` imports there.
- **`#![deny(unsafe_code)]` in core:** Unsafe code is only permitted in
  `crates/ffi` with documented `// SAFETY:` comments.
- **All public items must have `///` doc comments** with at minimum a summary
  line.
- **Hot-path functions** (`resonate`, `transmit`, `decay`) must carry
  `#[inline(always)]`.
- **New features** must include tests and use the `Telemetry` trait hooks.
- **Confidence > Detection:** SNR and jitter metrics must accompany any new
  detection logic.

## Pull Request Process

1. Ensure CI passes (tests, fmt, clippy, audit)
2. Update `CHANGELOG.md` under an `[Unreleased]` section
3. Add or update tests for your changes
4. Keep PRs focused — one feature or fix per PR
5. Write a clear description of what and why

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation only
- `perf:` performance improvement
- `refactor:` code restructuring without behavior change
- `test:` adding or updating tests
- `chore:` build/CI/tooling changes

## Reporting Issues

- Use the GitHub issue templates (bug report or feature request)
- Include reproduction steps for bugs
- Include your Rust version (`rustc --version`) and OS

## License

By contributing, you agree that your contributions will be licensed under
the AGPL-3.0 license and that the copyright holder (Belkis Aslani) retains
the right to offer your contributions under the commercial license as well
(Contributor License Agreement).
