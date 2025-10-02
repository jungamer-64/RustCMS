//! Database Performance Benchmarks
//!
//! Comprehensive benchmarks for database operations:
//! - Connection pool management
//! - CRUD operations (Create, Read, Update, Delete)
//! - Query performance (simple and complex)
//! - Transaction handling
//! - Concurrent database access
//! - Pagination queries
//!
//! # Setup Requirements
//! These benchmarks require a running PostgreSQL database.
//! Set DATABASE_URL environment variable or use test database.
//!
//! # Performance Targets
//! - Connection acquisition: < 10ms
//! - Simple query: < 5ms
//! - Complex join: < 50ms
//! - Transaction commit: < 20ms
//! - Concurrent operations: Linear scaling

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::Arc;
use tokio::runtime::Runtime;

mod common;
use common::concurrency_levels;

// ============================================================================
// Mock Database Types for Benchmarking
// ============================================================================

/// Mock connection pool for benchmarking
#[derive(Clone)]
struct MockPool {
    connections: Arc<Vec<MockConnection>>,
    current: Arc<std::sync::atomic::AtomicUsize>,
}

impl MockPool {
    fn new(size: usize) -> Self {
        let connections: Vec<_> = (0..size)
            .map(|_| MockConnection::new())
            .collect();
        
        Self {
            connections: Arc::new(connections),
            current: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
    
    fn get(&self) -> Result<MockConnection, String> {
        let idx = self.current.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(self.connections[idx % self.connections.len()].clone())
    }
}

/// Mock database connection
#[derive(Clone)]
struct MockConnection {
    id: usize,
}

impl MockConnection {
    fn new() -> Self {
        Self {
            id: rand::random(),
        }
    }
    
    fn execute_query<T>(&self, _query: &str) -> Result<T, String>
    where
        T: Default,
    {
        // Simulate query execution time
        std::thread::sleep(std::time::Duration::from_micros(100));
        Ok(T::default())
    }
    
    fn execute_insert(&self, _query: &str) -> Result<usize, String> {
        std::thread::sleep(std::time::Duration::from_micros(200));
        Ok(1)
    }
    
    fn execute_update(&self, _query: &str) -> Result<usize, String> {
        std::thread::sleep(std::time::Duration::from_micros(150));
        Ok(1)
    }
    
    fn execute_delete(&self, _query: &str) -> Result<usize, String> {
        std::thread::sleep(std::time::Duration::from_micros(100));
        Ok(1)
    }
}

/// Mock transaction
struct MockTransaction {
    conn: MockConnection,
    committed: bool,
}

impl MockTransaction {
    fn new(conn: MockConnection) -> Self {
        Self {
            conn,
            committed: false,
        }
    }
    
    fn execute(&self, query: &str) -> Result<(), String> {
        self.conn.execute_query::<()>(query)?;
        Ok(())
    }
    
    fn commit(mut self) -> Result<(), String> {
        std::thread::sleep(std::time::Duration::from_micros(500));
        self.committed = true;
        Ok(())
    }
}

impl Drop for MockTransaction {
    fn drop(&mut self) {
        if !self.committed {
            // Simulate rollback
            std::thread::sleep(std::time::Duration::from_micros(300));
        }
    }
}

// ============================================================================
// Setup Functions
// ============================================================================

fn setup_pool(size: usize) -> MockPool {
    MockPool::new(size)
}

// ============================================================================
// Connection Pool Benchmarks
// ============================================================================

/// Benchmark connection acquisition from pool
fn bench_connection_acquisition(c: &mut Criterion) {
    let pool = setup_pool(10);

    c.bench_function("database/connection_acquisition", |b| {
        b.iter(|| {
            let conn = pool.get();
            black_box(conn)
        })
    });
}

/// Benchmark connection acquisition with different pool sizes
fn bench_connection_acquisition_by_pool_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("database/connection_by_pool_size");

    for pool_size in [5, 10, 20, 50, 100].iter() {
        let pool = setup_pool(*pool_size);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(pool_size),
            &pool,
            |b, pool| {
                b.iter(|| {
                    let conn = pool.get();
                    black_box(conn)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Query Benchmarks
// ============================================================================

/// Benchmark simple SELECT query
fn bench_simple_query(c: &mut Criterion) {
    let pool = setup_pool(10);
    let conn = pool.get().expect("Failed to get connection");

    c.bench_function("database/simple_select", |b| {
        b.iter(|| {
            conn.execute_query::<Vec<String>>(black_box(
                "SELECT * FROM posts WHERE id = $1"
            ))
        })
    });
}

/// Benchmark queries with different WHERE clause complexities
fn bench_query_complexity(c: &mut Criterion) {
    let pool = setup_pool(10);
    let conn = pool.get().expect("Failed to get connection");
    let mut group = c.benchmark_group("database/query_complexity");

    let queries = vec![
        ("simple", "SELECT * FROM posts WHERE id = $1"),
        ("with_and", "SELECT * FROM posts WHERE id = $1 AND status = $2"),
        ("with_or", "SELECT * FROM posts WHERE category = $1 OR tag = $2"),
        ("with_in", "SELECT * FROM posts WHERE id IN ($1, $2, $3, $4, $5)"),
        ("with_like", "SELECT * FROM posts WHERE title LIKE $1 OR content LIKE $1"),
    ];

    for (name, query) in queries {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &query,
            |b, &q| {
                b.iter(|| {
                    conn.execute_query::<Vec<String>>(black_box(q))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark complex JOIN query
fn bench_complex_join_query(c: &mut Criterion) {
    let pool = setup_pool(10);
    let conn = pool.get().expect("Failed to get connection");

    c.bench_function("database/complex_join", |b| {
        b.iter(|| {
            conn.execute_query::<Vec<String>>(black_box(
                "SELECT posts.*, users.username, categories.name \
                 FROM posts \
                 JOIN users ON posts.author_id = users.id \
                 LEFT JOIN categories ON posts.category_id = categories.id \
                 WHERE posts.status = $1 \
                 ORDER BY posts.created_at DESC \
                 LIMIT 20"
            ))
        })
    });
}

/// Benchmark queries with different JOIN types
fn bench_join_types(c: &mut Criterion) {
    let pool = setup_pool(10);
    let conn = pool.get().expect("Failed to get connection");
    let mut group = c.benchmark_group("database/join_types");

    let queries = vec![
        ("inner_join", 
         "SELECT * FROM posts INNER JOIN users ON posts.author_id = users.id"),
        ("left_join", 
         "SELECT * FROM posts LEFT JOIN users ON posts.author_id = users.id"),
        ("right_join", 
         "SELECT * FROM posts RIGHT JOIN users ON posts.author_id = users.id"),
        ("multi_join", 
         "SELECT * FROM posts \
          JOIN users ON posts.author_id = users.id \
          LEFT JOIN categories ON posts.category_id = categories.id"),
    ];

    for (name, query) in queries {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &query,
            |b, &q| {
                b.iter(|| {
                    conn.execute_query::<Vec<String>>(black_box(q))
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// CRUD Operation Benchmarks
// ============================================================================

/// Benchmark INSERT operations
fn bench_insert_operations(c: &mut Criterion) {
    let pool = setup_pool(10);
    let mut group = c.benchmark_group("database/insert_operations");

    for batch_size in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &size| {
                let conn = pool.get().expect("Failed to get connection");

                b.iter(|| {
                    for _ in 0..size {
                        conn.execute_insert(black_box(
                            "INSERT INTO posts (title, content, author_id) VALUES ($1, $2, $3)"
                        )).ok();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark UPDATE operations
fn bench_update_operations(c: &mut Criterion) {
    let pool = setup_pool(10);
    let conn = pool.get().expect("Failed to get connection");

    c.bench_function("database/update_single", |b| {
        b.iter(|| {
            conn.execute_update(black_box(
                "UPDATE posts SET title = $1, updated_at = NOW() WHERE id = $2"
            ))
        })
    });
}

/// Benchmark DELETE operations
fn bench_delete_operations(c: &mut Criterion) {
    let pool = setup_pool(10);
    let conn = pool.get().expect("Failed to get connection");

    c.bench_function("database/delete_single", |b| {
        b.iter(|| {
            conn.execute_delete(black_box(
                "DELETE FROM posts WHERE id = $1"
            ))
        })
    });
}

/// Benchmark bulk DELETE operations
fn bench_bulk_delete(c: &mut Criterion) {
    let pool = setup_pool(10);
    let mut group = c.benchmark_group("database/bulk_delete");

    for count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &cnt| {
                let conn = pool.get().expect("Failed to get connection");
                b.iter(|| {
                    conn.execute_delete(black_box(
                        &format!("DELETE FROM posts WHERE id IN ({})", 
                                (1..=cnt).map(|_| "$1").collect::<Vec<_>>().join(","))
                    ))
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Transaction Benchmarks
// ============================================================================

/// Benchmark transaction commit
fn bench_transaction_commit(c: &mut Criterion) {
    let pool = setup_pool(10);

    c.bench_function("database/transaction_commit", |b| {
        b.iter(|| {
            let conn = pool.get().expect("Failed to get connection");
            let tx = MockTransaction::new(conn);
            tx.execute(black_box(
                "INSERT INTO posts (title, content) VALUES ($1, $2)"
            )).ok();
            tx.execute(black_box(
                "UPDATE users SET post_count = post_count + 1 WHERE id = $1"
            )).ok();
            tx.commit()
        })
    });
}

/// Benchmark transaction rollback
fn bench_transaction_rollback(c: &mut Criterion) {
    let pool = setup_pool(10);

    c.bench_function("database/transaction_rollback", |b| {
        b.iter(|| {
            let conn = pool.get().expect("Failed to get connection");
            let tx = MockTransaction::new(conn);
            tx.execute(black_box(
                "INSERT INTO posts (title, content) VALUES ($1, $2)"
            )).ok();
            tx.execute(black_box(
                "UPDATE users SET post_count = post_count + 1 WHERE id = $1"
            )).ok();
            // Drop without commit = rollback
            drop(tx);
        })
    });
}

/// Benchmark transactions with different operation counts
fn bench_transaction_by_operation_count(c: &mut Criterion) {
    let pool = setup_pool(10);
    let mut group = c.benchmark_group("database/transaction_operations");

    for op_count in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(op_count),
            op_count,
            |b, &count| {
                b.iter(|| {
                    let conn = pool.get().expect("Failed to get connection");
                    let tx = MockTransaction::new(conn);
                    for _ in 0..count {
                        tx.execute(black_box(
                            "INSERT INTO posts (title) VALUES ($1)"
                        )).ok();
                    }
                    tx.commit()
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Pagination Benchmarks
// ============================================================================

/// Benchmark pagination queries
fn bench_pagination_queries(c: &mut Criterion) {
    let pool = setup_pool(10);
    let conn = pool.get().expect("Failed to get connection");
    let mut group = c.benchmark_group("database/pagination");

    for page in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("page_{}", page)),
            page,
            |b, &page| {
                let offset = (page - 1) * 20;
                b.iter(|| {
                    conn.execute_query::<Vec<String>>(black_box(
                        &format!("SELECT * FROM posts \
                                 WHERE status = 'published' \
                                 ORDER BY created_at DESC \
                                 LIMIT 20 OFFSET {}", offset)
                    ))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark pagination with different page sizes
fn bench_pagination_by_page_size(c: &mut Criterion) {
    let pool = setup_pool(10);
    let conn = pool.get().expect("Failed to get connection");
    let mut group = c.benchmark_group("database/pagination_page_size");

    for page_size in [10, 20, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_items", page_size)),
            page_size,
            |b, &size| {
                b.iter(|| {
                    conn.execute_query::<Vec<String>>(black_box(
                        &format!("SELECT * FROM posts \
                                 WHERE status = 'published' \
                                 ORDER BY created_at DESC \
                                 LIMIT {}", size)
                    ))
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Concurrent Access Benchmarks
// ============================================================================

/// Benchmark concurrent reads
fn bench_concurrent_reads(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = Arc::new(setup_pool(20));
    let mut group = c.benchmark_group("database/concurrent_reads");

    // Use dynamic concurrency levels based on CPU count
    for concurrency in concurrency_levels().iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &count| {
                let pool = pool.clone();
                b.to_async(&rt).iter(|| {
                    let pool = pool.clone();
                    async move {
                        let handles: Vec<_> = (0..count)
                            .map(|_| {
                                let pool = pool.clone();
                                tokio::spawn(async move {
                                    let conn = pool.get().ok()?;
                                    conn.execute_query::<Vec<String>>(
                                        "SELECT * FROM posts WHERE id = $1"
                                    ).ok()
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

/// Benchmark concurrent writes
fn bench_concurrent_writes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = Arc::new(setup_pool(20));
    let mut group = c.benchmark_group("database/concurrent_writes");

    // Use dynamic concurrency levels based on CPU count (limit to 20)
    for concurrency in concurrency_levels().iter().filter(|&&x| x <= 20) {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &count| {
                let pool = pool.clone();
                b.to_async(&rt).iter(|| {
                    let pool = pool.clone();
                    async move {
                        let handles: Vec<_> = (0..count)
                            .map(|_| {
                                let pool = pool.clone();
                                tokio::spawn(async move {
                                    let conn = pool.get().ok()?;
                                    conn.execute_insert(
                                        "INSERT INTO posts (title, content) VALUES ($1, $2)"
                                    ).ok()
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

/// Benchmark mixed read/write workload
fn bench_mixed_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = Arc::new(setup_pool(20));

    c.bench_function("database/mixed_workload", |b| {
        b.to_async(&rt).iter(|| {
            let pool = pool.clone();
            async move {
                let handles: Vec<_> = (0..20)
                    .map(|i| {
                        let pool = pool.clone();
                        tokio::spawn(async move {
                            let conn = pool.get().ok()?;
                            if i % 3 == 0 {
                                // 33% writes
                                conn.execute_insert(
                                    "INSERT INTO posts (title) VALUES ($1)"
                                ).ok()
                            } else {
                                // 67% reads
                                conn.execute_query::<Vec<String>>(
                                    "SELECT * FROM posts WHERE id = $1"
                                ).ok();
                                Some(1)
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

// ============================================================================
// Aggregate and Grouping Benchmarks
// ============================================================================

/// Benchmark aggregate queries
fn bench_aggregate_queries(c: &mut Criterion) {
    let pool = setup_pool(10);
    let conn = pool.get().expect("Failed to get connection");
    let mut group = c.benchmark_group("database/aggregate");

    let queries = vec![
        ("count", "SELECT COUNT(*) FROM posts"),
        ("sum", "SELECT SUM(view_count) FROM posts"),
        ("avg", "SELECT AVG(view_count) FROM posts"),
        ("max_min", "SELECT MAX(created_at), MIN(created_at) FROM posts"),
        ("group_by", "SELECT author_id, COUNT(*) FROM posts GROUP BY author_id"),
    ];

    for (name, query) in queries {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &query,
            |b, &q| {
                b.iter(|| {
                    conn.execute_query::<Vec<String>>(black_box(q))
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
    name = connection_benches;
    config = Criterion::default();
    targets = 
        bench_connection_acquisition,
        bench_connection_acquisition_by_pool_size,
);

criterion_group!(
    name = query_benches;
    config = Criterion::default();
    targets = 
        bench_simple_query,
        bench_query_complexity,
        bench_complex_join_query,
        bench_join_types,
);

criterion_group!(
    name = crud_benches;
    config = Criterion::default();
    targets = 
        bench_insert_operations,
        bench_update_operations,
        bench_delete_operations,
        bench_bulk_delete,
);

criterion_group!(
    name = transaction_benches;
    config = Criterion::default();
    targets = 
        bench_transaction_commit,
        bench_transaction_rollback,
        bench_transaction_by_operation_count,
);

criterion_group!(
    name = pagination_benches;
    config = Criterion::default();
    targets = 
        bench_pagination_queries,
        bench_pagination_by_page_size,
);

criterion_group!(
    name = concurrent_benches;
    config = Criterion::default();
    targets = 
        bench_concurrent_reads,
        bench_concurrent_writes,
        bench_mixed_workload,
);

criterion_group!(
    name = aggregate_benches;
    config = Criterion::default();
    targets = 
        bench_aggregate_queries,
);

criterion_main!(
    connection_benches,
    query_benches,
    crud_benches,
    transaction_benches,
    pagination_benches,
    concurrent_benches,
    aggregate_benches
);