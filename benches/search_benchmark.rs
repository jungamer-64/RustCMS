//! Search Engine Performance Benchmarks
//!
//! Comprehensive benchmarks for search operations:
//! - Document indexing (single and bulk)
//! - Search queries (simple and complex)
//! - Result ranking and scoring
//! - Pagination performance
//! - Index maintenance operations
//!
//! # Performance Targets
//! - Single document indexing: < 10ms
//! - Bulk indexing (100 docs): < 500ms
//! - Simple search: < 50ms
//! - Complex search: < 200ms
//! - Pagination: Linear time complexity

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cms_backend::search::{SearchEngine, SearchQuery, Document};
use tempfile::TempDir;
use uuid::Uuid;
use chrono::Utc;

mod common;
use common::{generate_test_content, generate_test_tags};

// ============================================================================
// Setup and Configuration
// ============================================================================

/// Create search engine instance for benchmarking with proper cleanup
fn create_search_engine() -> (SearchEngine, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let search_engine = SearchEngine::new(temp_dir.path().to_str().unwrap())
        .expect("Failed to create search engine");
    (search_engine, temp_dir)
}

/// Generate test document
fn create_test_document(id: usize) -> Document {
    Document {
        id: Uuid::new_v4(),
        title: format!("Document {} about Rust and Performance", id),
        content: format!(
            "This is document number {}. It contains comprehensive information about \
             Rust programming language, performance optimization, and modern software \
             development practices. Topics include memory management, concurrency, \
             type systems, and efficient algorithms.",
            id
        ),
        excerpt: Some(format!("Excerpt for document {}", id)),
        tags: vec!["rust".to_string(), "performance".to_string(), "programming".to_string()],
        author: format!("author_{}", id % 10),
        created_at: Utc::now(),
    }
}

/// Generate document with custom content
fn create_custom_document(id: usize, title: &str, content: &str, tags: Vec<String>) -> Document {
    Document {
        id: Uuid::new_v4(),
        title: title.to_string(),
        content: content.to_string(),
        excerpt: Some(format!("Excerpt: {}", title)),
        tags,
        author: format!("author_{}", id % 10),
        created_at: Utc::now(),
    }
}

/// Generate large document (for testing different sizes) - uses common utility
fn create_large_document(id: usize, content_size: usize) -> Document {
    let content = generate_test_content(content_size);
    let tags = generate_test_tags(2);
    
    Document {
        id: Uuid::new_v4(),
        title: format!("Large Document {}", id),
        content,
        excerpt: Some(format!("Large document excerpt {}", id)),
        tags,
        author: "benchmark_author".to_string(),
        created_at: Utc::now(),
    }
}

/// Populate search engine with test documents
fn populate_search_engine(engine: &mut SearchEngine, count: usize) {
    for i in 0..count {
        let doc = create_test_document(i);
        engine.index_document(&doc).ok();
    }
    engine.commit().ok();
}

// ============================================================================
// Document Indexing Benchmarks
// ============================================================================

/// Benchmark single document indexing
fn bench_document_indexing(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = create_search_engine();
    let document = create_test_document(0);

    c.bench_function("search/index_single_document", |b| {
        b.iter(|| {
            search_engine.index_document(black_box(&document))
        })
    });
}

/// Benchmark indexing documents of different sizes
fn bench_document_indexing_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("search/index_by_size");
    
    for size in [100, 500, 1000, 5000, 10000].iter() {
        let (mut search_engine, _temp_dir) = create_search_engine();
        let document = create_large_document(0, *size);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &document,
            |b, doc| {
                b.iter(|| search_engine.index_document(black_box(doc)));
            },
        );
    }
    
    group.finish();
}

/// Benchmark bulk document indexing
fn bench_bulk_indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("search/bulk_indexing");
    
    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let (mut search_engine, _temp_dir) = create_search_engine();
                let documents: Vec<_> = (0..size)
                    .map(|i| create_test_document(i))
                    .collect();

                b.iter(|| {
                    for doc in &documents {
                        let _ = search_engine.index_document(black_box(doc));
                    }
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark bulk indexing with commit
fn bench_bulk_indexing_with_commit(c: &mut Criterion) {
    let mut group = c.benchmark_group("search/bulk_indexing_commit");
    
    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let documents: Vec<_> = (0..size)
                    .map(|i| create_test_document(i))
                    .collect();

                b.iter(|| {
                    let (mut search_engine, _temp_dir) = create_search_engine();
                    for doc in &documents {
                        let _ = search_engine.index_document(doc);
                    }
                    search_engine.commit()
                });
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Search Query Benchmarks
// ============================================================================

/// Benchmark simple search query
fn bench_simple_search(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = create_search_engine();
    populate_search_engine(&mut search_engine, 100);

    let query = SearchQuery {
        query: "rust programming".to_string(),
        page: 1,
        per_page: 10,
    };

    c.bench_function("search/simple_query", |b| {
        b.iter(|| search_engine.search(black_box(&query)))
    });
}

/// Benchmark search with different query lengths
fn bench_search_by_query_length(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = create_search_engine();
    populate_search_engine(&mut search_engine, 200);
    
    let mut group = c.benchmark_group("search/query_by_length");
    
    let queries = vec![
        ("single_word", "rust"),
        ("two_words", "rust programming"),
        ("phrase", "rust programming language"),
        ("long_query", "rust programming language performance optimization modern development"),
    ];

    for (name, query_text) in queries {
        let query = SearchQuery {
            query: query_text.to_string(),
            page: 1,
            per_page: 10,
        };
        
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &query,
            |b, q| {
                b.iter(|| search_engine.search(black_box(q)));
            },
        );
    }
    
    group.finish();
}

/// Benchmark complex search query
fn bench_complex_search(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = create_search_engine();
    
    // Index documents with varied content for complex queries
    for i in 0..200 {
        let doc = create_custom_document(
            i,
            &format!("Article about {} and {}", 
                if i % 2 == 0 { "performance" } else { "security" },
                if i % 3 == 0 { "optimization" } else { "best practices" }
            ),
            &format!("Detailed content about various aspects of software development. Index: {}", i),
            vec![
                "rust".to_string(),
                if i % 2 == 0 { "performance" } else { "security" }.to_string(),
            ],
        );
        search_engine.index_document(&doc).ok();
    }
    search_engine.commit().ok();

    let query = SearchQuery {
        query: "performance optimization best practices".to_string(),
        page: 1,
        per_page: 20,
    };

    c.bench_function("search/complex_query", |b| {
        b.iter(|| search_engine.search(black_box(&query)))
    });
}

/// Benchmark search with different result set sizes
fn bench_search_by_result_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("search/query_by_result_size");
    
    for doc_count in [50, 100, 500, 1000].iter() {
        let (mut search_engine, _temp_dir) = create_search_engine();
        populate_search_engine(&mut search_engine, *doc_count);
        
        let query = SearchQuery {
            query: "rust programming".to_string(),
            page: 1,
            per_page: 10,
        };
        
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_docs", doc_count)),
            &query,
            |b, q| {
                b.iter(|| search_engine.search(black_box(q)));
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Pagination Benchmarks
// ============================================================================

/// Benchmark search pagination
fn bench_search_pagination(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = create_search_engine();
    populate_search_engine(&mut search_engine, 500);

    let mut group = c.benchmark_group("search/pagination");

    for page in [1, 5, 10, 20, 50].iter() {
        let query = SearchQuery {
            query: "rust programming".to_string(),
            page: *page,
            per_page: 10,
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("page_{}", page)),
            &query,
            |b, q| {
                b.iter(|| search_engine.search(black_box(q)));
            },
        );
    }

    group.finish();
}

/// Benchmark pagination with different page sizes
fn bench_pagination_by_page_size(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = create_search_engine();
    populate_search_engine(&mut search_engine, 500);

    let mut group = c.benchmark_group("search/pagination_page_size");

    for per_page in [5, 10, 20, 50, 100].iter() {
        let query = SearchQuery {
            query: "rust programming".to_string(),
            page: 1,
            per_page: *per_page,
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_per_page", per_page)),
            &query,
            |b, q| {
                b.iter(|| search_engine.search(black_box(q)));
            },
        );
    }

    group.finish();
}

// ============================================================================
// Index Maintenance Benchmarks
// ============================================================================

/// Benchmark index commit operation
fn bench_index_commit(c: &mut Criterion) {
    let mut group = c.benchmark_group("search/index_commit");
    
    for doc_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_docs", doc_count)),
            doc_count,
            |b, &count| {
                b.iter(|| {
                    let (mut search_engine, _temp_dir) = create_search_engine();
                    for i in 0..count {
                        let doc = create_test_document(i);
                        search_engine.index_document(&doc).ok();
                    }
                    search_engine.commit()
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark index update (reindexing existing document)
fn bench_index_update(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = create_search_engine();
    let doc_id = Uuid::new_v4();
    
    let document = Document {
        id: doc_id,
        title: "Original Title".to_string(),
        content: "Original content".to_string(),
        excerpt: Some("Original excerpt".to_string()),
        tags: vec!["tag1".to_string()],
        author: "author".to_string(),
        created_at: Utc::now(),
    };
    
    search_engine.index_document(&document).ok();
    search_engine.commit().ok();

    let updated_document = Document {
        id: doc_id,
        title: "Updated Title".to_string(),
        content: "Updated content with more information".to_string(),
        excerpt: Some("Updated excerpt".to_string()),
        tags: vec!["tag1".to_string(), "tag2".to_string()],
        author: "author".to_string(),
        created_at: Utc::now(),
    };

    c.bench_function("search/index_update", |b| {
        b.iter(|| {
            search_engine.index_document(black_box(&updated_document))
        })
    });
}

// ============================================================================
// Real-world Scenario Benchmarks
// ============================================================================

/// Benchmark mixed indexing and search workload
fn bench_mixed_workload(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = create_search_engine();
    populate_search_engine(&mut search_engine, 100);

    c.bench_function("search/mixed_workload", |b| {
        b.iter(|| {
            // 70% search, 30% indexing (realistic ratio)
            for i in 0..10 {
                if i < 7 {
                    // Search
                    let query = SearchQuery {
                        query: "rust programming".to_string(),
                        page: 1,
                        per_page: 10,
                    };
                    let _ = search_engine.search(&query);
                } else {
                    // Index new document
                    let doc = create_test_document(i + 100);
                    let _ = search_engine.index_document(&doc);
                }
            }
        });
    });
}

/// Benchmark search with varied query patterns
fn bench_varied_query_patterns(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = create_search_engine();
    populate_search_engine(&mut search_engine, 200);

    c.bench_function("search/varied_queries", |b| {
        b.iter(|| {
            let queries = vec![
                "rust",
                "programming language",
                "performance optimization",
                "memory management concurrency",
                "modern software development",
            ];
            
            for query_text in queries {
                let query = SearchQuery {
                    query: query_text.to_string(),
                    page: 1,
                    per_page: 10,
                };
                let _ = search_engine.search(black_box(&query));
            }
        });
    });
}

// ============================================================================
// Benchmark Group Configuration
// ============================================================================

criterion_group!(
    name = indexing_benches;
    config = Criterion::default();
    targets = 
        bench_document_indexing,
        bench_document_indexing_by_size,
        bench_bulk_indexing,
        bench_bulk_indexing_with_commit,
);

criterion_group!(
    name = search_benches;
    config = Criterion::default();
    targets = 
        bench_simple_search,
        bench_search_by_query_length,
        bench_complex_search,
        bench_search_by_result_size,
);

criterion_group!(
    name = pagination_benches;
    config = Criterion::default();
    targets = 
        bench_search_pagination,
        bench_pagination_by_page_size,
);

criterion_group!(
    name = maintenance_benches;
    config = Criterion::default();
    targets = 
        bench_index_commit,
        bench_index_update,
);

criterion_group!(
    name = scenario_benches;
    config = Criterion::default();
    targets = 
        bench_mixed_workload,
        bench_varied_query_patterns,
);

criterion_main!(
    indexing_benches,
    search_benches,
    pagination_benches,
    maintenance_benches,
    scenario_benches
);