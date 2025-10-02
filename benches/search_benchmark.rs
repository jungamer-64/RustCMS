//! Search Performance Benchmarks - TEMPORARILY DISABLED

use criterion::{criterion_group, criterion_main, Criterion};

fn stub_benchmark(c: &mut Criterion) {
    c.bench_function("search/stub_placeholder", |b| {
        b.iter(|| std::hint::black_box(42))
    });
}

criterion_group!(search_benches, stub_benchmark);
criterion_main!(search_benches);
