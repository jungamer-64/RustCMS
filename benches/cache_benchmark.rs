//! Cache Performance Benchmarks
//!
//! Benchmarks for:
//! - Cache set operations
//! - Cache get operations
//! - Cache eviction
//! - Concurrent access

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cms_backend::cache::{Cache, CacheKey, CacheValue};
use std::sync::Arc;

fn setup_cache() -> Cache {
    Cache::new(1000, 3600) // 1000 entries, 1 hour TTL
}

fn bench_cache_set(c: &mut Criterion) {
    let cache = setup_cache();
    let key = CacheKey::from("test_key");
    let value = CacheValue::String("test_value".to_string());

    c.bench_function("cache_set", |b| {
        b.iter(|| {
            cache.set(black_box(&key), black_box(value.clone()))
        })
    });
}

fn bench_cache_get_hit(c: &mut Criterion) {
    let cache = setup_cache();
    let key = CacheKey::from("test_key");
    let value = CacheValue::String("test_value".to_string());
    cache.set(&key, value).ok();

    c.bench_function("cache_get_hit", |b| {
        b.iter(|| cache.get(black_box(&key)))
    });
}

fn bench_cache_get_miss(c: &mut Criterion) {
    let cache = setup_cache();
    let key = CacheKey::from("nonexistent_key");

    c.bench_function("cache_get_miss", |b| {
        b.iter(|| cache.get(black_box(&key)))
    });
}

fn bench_cache_bulk_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_bulk_operations");

    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let cache = setup_cache();

            b.iter(|| {
                for i in 0..size {
                    let key = CacheKey::from(format!("key_{}", i));
                    let value = CacheValue::String(format!("value_{}", i));
                    cache.set(black_box(&key), black_box(value)).ok();
                }
            });
        });
    }

    group.finish();
}

fn bench_cache_eviction(c: &mut Criterion) {
    let cache = setup_cache();

    // Fill cache beyond capacity to trigger eviction
    for i in 0..1500 {
        let key = CacheKey::from(format!("key_{}", i));
        let value = CacheValue::String(format!("value_{}", i));
        cache.set(&key, value).ok();
    }

    c.bench_function("cache_eviction", |b| {
        b.iter(|| {
            let key = CacheKey::from(format!("key_{}", rand::random::<u32>()));
            let value = CacheValue::String("new_value".to_string());
            cache.set(black_box(&key), black_box(value))
        })
    });
}

fn bench_concurrent_cache_access(c: &mut Criterion) {
    use tokio::runtime::Runtime;

    let rt = Runtime::new().unwrap();
    let cache = Arc::new(setup_cache());

    // Pre-populate cache
    for i in 0..100 {
        let key = CacheKey::from(format!("key_{}", i));
        let value = CacheValue::String(format!("value_{}", i));
        cache.set(&key, value).ok();
    }

    c.bench_function("concurrent_cache_reads_10", |b| {
        b.to_async(&rt).iter(|| {
            let cache = cache.clone();
            async move {
                let handles: Vec<_> = (0..10)
                    .map(|i| {
                        let cache = cache.clone();
                        tokio::spawn(async move {
                            let key = CacheKey::from(format!("key_{}", i % 100));
                            cache.get(&key)
                        })
                    })
                    .collect();

                for handle in handles {
                    let _ = handle.await;
                }
            }
        });
    });
}

fn bench_concurrent_cache_writes(c: &mut Criterion) {
    use tokio::runtime::Runtime;

    let rt = Runtime::new().unwrap();
    let cache = Arc::new(setup_cache());

    c.bench_function("concurrent_cache_writes_10", |b| {
        b.to_async(&rt).iter(|| {
            let cache = cache.clone();
            async move {
                let handles: Vec<_> = (0..10)
                    .map(|i| {
                        let cache = cache.clone();
                        tokio::spawn(async move {
                            let key = CacheKey::from(format!("key_{}", i));
                            let value = CacheValue::String(format!("value_{}", i));
                            cache.set(&key, value)
                        })
                    })
                    .collect();

                for handle in handles {
                    let _ = handle.await;
                }
            }
        });
    });
}

fn bench_cache_mixed_workload(c: &mut Criterion) {
    use tokio::runtime::Runtime;
    use rand::Rng;

    let rt = Runtime::new().unwrap();
    let cache = Arc::new(setup_cache());

    // Pre-populate cache
    for i in 0..100 {
        let key = CacheKey::from(format!("key_{}", i));
        let value = CacheValue::String(format!("value_{}", i));
        cache.set(&key, value).ok();
    }

    c.bench_function("cache_mixed_workload", |b| {
        b.to_async(&rt).iter(|| {
            let cache = cache.clone();
            async move {
                let handles: Vec<_> = (0..20)
                    .map(|_| {
                        let cache = cache.clone();
                        tokio::spawn(async move {
                            let mut rng = rand::thread_rng();
                            let operation = rng.gen_range(0..10);
                            
                            if operation < 7 {
                                // 70% reads
                                let key = CacheKey::from(format!("key_{}", rng.gen_range(0..100)));
                                cache.get(&key)
                            } else {
                                // 30% writes
                                let key = CacheKey::from(format!("key_{}", rng.gen_range(0..100)));
                                let value = CacheValue::String(format!("value_{}", rng.gen_range(0..u32::MAX)));
                                cache.set(&key, value).ok();
                                Ok(None)
                            }
                        })
                    })
                    .collect();

                for handle in handles {
                    let _ = handle.await;
                }
            }
        });
    });
}

fn bench_cache_ttl_operations(c: &mut Criterion) {
    let cache = setup_cache();
    let key = CacheKey::from("ttl_key");
    let value = CacheValue::String("ttl_value".to_string());

    c.bench_function("cache_set_with_ttl", |b| {
        b.iter(|| {
            cache.set_with_ttl(
                black_box(&key),
                black_box(value.clone()),
                black_box(60), // 60 seconds TTL
            )
        })
    });
}

criterion_group!(
    benches,
    bench_cache_set,
    bench_cache_get_hit,
    bench_cache_get_miss,
    bench_cache_bulk_operations,
    bench_cache_eviction,
    bench_concurrent_cache_access,
    bench_concurrent_cache_writes,
    bench_cache_mixed_workload,
    bench_cache_ttl_operations
);
criterion_main!(benches);
