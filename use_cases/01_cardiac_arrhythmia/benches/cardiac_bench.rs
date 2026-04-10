// SPDX-License-Identifier: AGPL-3.0-only
//! Placeholder benchmark — will be implemented in Run 4.

use criterion::{criterion_group, criterion_main, Criterion};

fn cardiac_placeholder(c: &mut Criterion) {
    c.bench_function("cardiac_noop", |b| b.iter(|| 1 + 1));
}

criterion_group!(benches, cardiac_placeholder);
criterion_main!(benches);
