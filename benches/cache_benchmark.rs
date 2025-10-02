//! Cache Performance Benchmarks
//!
//! Comprehensive benchmarks for cache operations:
//! - Set operations (single and bulk)
//! - Get operations (hits and misses)
//! - Cache eviction strategies
//! - Concurrent access patterns
//! - TTL operations
//! - Mixed read/write workloads
//!
//! # Performance Targets
//! - Single set/get: < 1ms
//! - Bulk operations: Linear scaling
//! - Concurrent access: Minimal lock contention
//! - Eviction: Minimal impact on throughput

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cms_backend::cache::{Cache, CacheKey, CacheValue};
use std::sync::Arc;
use tokio::runtime::Runtime;
use rand::Rng;

mod common;
use common::{concurrency_levels, handle_bench_error};

// ============================================================================
// Setup and Configuration
// ============================================================================

/// Create cache instance for benchmarking
fn create_cache() -> Cache {
    Cache::new(1000, 3600) // 1000 entries, 1 hour TTL
}

/// Create large cache for eviction testing
fn create_large_cache() -> Cache {
    Cache::new(10000, 3600)
}

/// Generate cache key
fn generate_cache_key(id: usize) -> CacheKey {
    CacheKey::from(format!("key_{}", id))
}

/// Generate cache value (string)
fn generate_string_value(id: usize) -> CacheValue {
    CacheValue::String(format!("value_{}", id))
}

/// Generate cache value (JSON-like data)
fn generate_json_value(id: usize) -> CacheValue {
    CacheValue::String(format!(
        r#"{{"id": {}, "name": "item_{}", "active": true}}"#,
        id, id
    ))
}

/// Pre-populate cache with test data
fn populate_cache(cache: &Cache, count: usize) {
    for i in 0..count {
        let key = generate_cache_key(i);
        let value = generate_string_value(i);
        cache.set(&key, value).ok();
    }
}

// ============================================================================
// Basic Set Operations
// ============================================================================

/// Benchmark single cache set operation
fn bench_cache_set(c: &mut Criterion) {
    let cache = create_cache();
    let key = generate_cache_key(0);
    let value = generate_string_value(0);

    c.bench_function("cache/set_single", |b| {
        b.iter(|| {
            cache.set(black_box(&key), black_box(value.clone()))
        })
    });
}

/// Benchmark cache set with different value sizes
fn bench_cache_set_by_size(c: &mut Criterion) {
    let cache = create_cache();
    let mut group = c.benchmark_group("cache/set_by_size");
    
    for size in [10, 100, 1000, 10000].iter() {
        let value = CacheValue::String("x".repeat(*size));
        let key = generate_cache_key(0);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &value,
            |b, val| {
                b.iter(|| cache.set(black_box(&key), black_box(val.clone())));
            },
        );
    }
    
    group.finish();
}

/// Benchmark cache set with TTL
fn bench_cache_set_with_ttl(c: &mut Criterion) {
    let cache = create_cache();
    let key = generate_cache_key(0);
    let value = generate_string_value(0);

    c.bench_function("cache/set_with_ttl", |b| {
        b.iter(|| {
            cache.set_with_ttl(
                black_box(&key),
                black_box(value.clone()),
                black_box(60), // 60 seconds TTL
            )
        })
    });
}

/// Benchmark cache set with varying TTL values
fn bench_cache_set_by_ttl(c: &mut Criterion) {
    let cache = create_cache();
    let mut group = c.benchmark_group("cache/set_by_ttl");
    let key = generate_cache_key(0);
    let value = generate_string_value(0);
    
    for ttl in [10, 60, 300, 3600].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(ttl),
            ttl,
            |b, &ttl_seconds| {
                b.iter(|| {
                    cache.set_with_ttl(
                        black_box(&key),
                        black_box(value.clone()),
                        black_box(ttl_seconds),
                    )
                });
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Basic Get Operations
// ============================================================================

/// Benchmark cache get with hit
fn bench_cache_get_hit(c: &mut Criterion) {
    let cache = create_cache();
    let key = generate_cache_key(0);
    let value = generate_string_value(0);
    cache.set(&key, value).ok();

    c.bench_function("cache/get_hit", |b| {
        b.iter(|| cache.get(black_box(&key)))
    });
}

/// Benchmark cache get with miss
fn bench_cache_get_miss(c: &mut Criterion) {
    let cache = create_cache();
    let key = generate_cache_key(999999);

    c.bench_function("cache/get_miss", |b| {
        b.iter(|| cache.get(black_box(&key)))
    });
}

/// Benchmark cache hit ratio impact
fn bench_cache_hit_ratios(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/hit_ratios");
    
    for hit_ratio in [0, 25, 50, 75, 100].iter() {
        let cache = create_cache();
        populate_cache(&cache, 100);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}%", hit_ratio)),
            hit_ratio,
            |b, &ratio| {
                b.iter(|| {
                    let key_id = if rand::random::<u8>() < (ratio as u8 * 255 / 100) {
                        rand::thread_rng().gen_range(0..100) // Hit
                    } else {
                        rand::thread_rng().gen_range(100..200) // Miss
                    };
                    let key = generate_cache_key(key_id);
                    cache.get(black_box(&key))
                });
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Bulk Operations
// ============================================================================

/// Benchmark bulk set operations
fn bench_cache_bulk_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/bulk_set");

    for size in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let cache = create_cache();
                b.iter(|| {
                    for i in 0..size {
                        let key = generate_cache_key(i);
                        let value = generate_string_value(i);
                        cache.set(black_box(&key), black_box(value)).ok();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark bulk get operations
fn bench_cache_bulk_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/bulk_get");

    for size in [10, 50, 100, 500, 1000].iter() {
        let cache = create_cache();
        populate_cache(&cache, *size);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                b.iter(|| {
                    for i in 0..size {
                        let key = generate_cache_key(i);
                        cache.get(black_box(&key));
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Eviction Benchmarks
// ============================================================================

/// Benchmark cache eviction behavior
fn bench_cache_eviction(c: &mut Criterion) {
    let cache = create_cache();

    // Fill cache beyond capacity to trigger eviction
    for i in 0..1500 {
        let key = generate_cache_key(i);
        let value = generate_string_value(i);
        cache.set(&key, value).ok();
    }

    c.bench_function("cache/eviction_active", |b| {
        b.iter(|| {
            let key = generate_cache_key(rand::random::<usize>());
            let value = generate_string_value(rand::random::<usize>());
            cache.set(black_box(&key), black_box(value))
        })
    });
}

/// Benchmark eviction with different cache sizes
fn bench_cache_eviction_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/eviction_by_size");
    
    for capacity in [100, 500, 1000, 5000].iter() {
        let cache = Cache::new(*capacity, 3600);
        
        // Fill to 150% capacity
        let fill_count = capacity * 3 / 2;
        for i in 0..fill_count {
            let key = generate_cache_key(i);
            let value = generate_string_value(i);
            cache.set(&key, value).ok();
        }
        
        group.bench_with_input(
            BenchmarkId::from_parameter(capacity),
            &cache,
            |b, cache| {
                b.iter(|| {
                    let key = generate_cache_key(rand::random::<usize>());
                    let value = generate_string_value(rand::random::<usize>());
                    cache.set(black_box(&key), black_box(value))
                });
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Concurrent Access Benchmarks
// ============================================================================

/// Benchmark concurrent cache reads
fn bench_concurrent_cache_reads(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = Arc::new(create_cache());
    populate_cache(&cache, 100);
    
    let mut group = c.benchmark_group("cache/concurrent_reads");

    // Use dynamic concurrency levels based on CPU count
    for concurrency in concurrency_levels().iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &count| {
                let cache = cache.clone();
                b.to_async(&rt).iter(|| {
                    let cache = cache.clone();
                    async move {
                        let handles: Vec<_> = (0..count)
                            .map(|i| {
                                let cache = cache.clone();
                                tokio::spawn(async move {
                                    let key = generate_cache_key(i % 100);
                                    cache.get(&key)
                                })
                            })
                            .collect();

                        for handle in handles {
                            let _ = handle.await;
                        }
                    }
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark concurrent cache writes
fn bench_concurrent_cache_writes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = Arc::new(create_cache());
    
    let mut group = c.benchmark_group("cache/concurrent_writes");

    // Use dynamic concurrency levels based on CPU count
    for concurrency in concurrency_levels().iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &count| {
                let cache = cache.clone();
                b.to_async(&rt).iter(|| {
                    let cache = cache.clone();
                    async move {
                        let handles: Vec<_> = (0..count)
                            .map(|i| {
                                let cache = cache.clone();
                                tokio::spawn(async move {
                                    let key = generate_cache_key(i);
                                    let value = generate_string_value(i);
                                    cache.set(&key, value)
                                })
                            })
                            .collect();

                        for handle in handles {
                            let _ = handle.await;
                        }
                    }
                });
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Mixed Workload Benchmarks
// ============================================================================

/// Benchmark mixed read/write workload
fn bench_cache_mixed_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = Arc::new(create_cache());
    populate_cache(&cache, 100);

    c.bench_function("cache/mixed_workload", |b| {
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
                                let key = generate_cache_key(rng.gen_range(0..100));
                                cache.get(&key)
                            } else {
                                // 30% writes
                                let key = generate_cache_key(rng.gen_range(0..100));
                                let value = generate_string_value(rng.gen_range(0..u32::MAX as usize));
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

/// Benchmark mixed workload with varying read/write ratios
fn bench_cache_workload_ratios(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache/workload_ratios");
    
    for read_percentage in [50, 70, 90, 95, 99].iter() {
        let cache = Arc::new(create_cache());
        populate_cache(&cache, 100);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}%_reads", read_percentage)),
            read_percentage,
            |b, &read_pct| {
                let cache = cache.clone();
                b.to_async(&rt).iter(|| {
                    let cache = cache.clone();
                    async move {
                        let handles: Vec<_> = (0..20)
                            .map(|_| {
                                let cache = cache.clone();
                                tokio::spawn(async move {
                                    let mut rng = rand::thread_rng();
                                    let operation = rng.gen_range(0..100);
                                    
                                    if operation < read_pct {
                                        // Read
                                        let key = generate_cache_key(rng.gen_range(0..100));
                                        cache.get(&key)
                                    } else {
                                        // Write
                                        let key = generate_cache_key(rng.gen_range(0..100));
                                        let value = generate_string_value(rng.gen_range(0..u32::MAX as usize));
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
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Benchmark Group Configuration
// ============================================================================

criterion_group!(
    name = basic_ops;
    config = Criterion::default();
    targets = 
        bench_cache_set,
        bench_cache_set_by_size,
        bench_cache_set_with_ttl,
        bench_cache_set_by_ttl,
        bench_cache_get_hit,
        bench_cache_get_miss,
        bench_cache_hit_ratios,
);

criterion_group!(
    name = bulk_ops;
    config = Criterion::default();
    targets = 
        bench_cache_bulk_set,
        bench_cache_bulk_get,
);

criterion_group!(
    name = eviction_ops;
    config = Criterion::default();
    targets = 
        bench_cache_eviction,
        bench_cache_eviction_by_size,
);

criterion_group!(
    name = concurrent_ops;
    config = Criterion::default();
    targets = 
        bench_concurrent_cache_reads,
        bench_concurrent_cache_writes,
);

criterion_group!(
    name = mixed_workload;
    config = Criterion::default();
    targets = 
        bench_cache_mixed_workload,
        bench_cache_workload_ratios,
);

criterion_main!(basic_ops, bulk_ops, eviction_ops, concurrent_ops, mixed_workload);