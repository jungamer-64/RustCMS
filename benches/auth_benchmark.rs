//! Authentication Performance Benchmarks
//!
//! TEMPORARILY DISABLED: This benchmark needs to be updated to work with the new `AuthService` API
//!
//! TODO: Rewrite benchmarks to use the current `AuthService` implementation

use criterion::{Criterion, criterion_group, criterion_main};

fn stub_benchmark(c: &mut Criterion) {
    c.bench_function("stub_auth_benchmark", |b| {
        b.iter(|| std::hint::black_box(42));
    });
}

criterion_group!(auth_benches, stub_benchmark);
criterion_main!(auth_benches);
