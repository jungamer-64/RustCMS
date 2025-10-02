//! Search Performance Benchmarks
//!
//! Benchmarks for:
//! - Index creation
//! - Document indexing
//! - Search queries
//! - Result ranking

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cms_backend::search::{SearchEngine, SearchQuery};
use tempfile::TempDir;
use uuid::Uuid;

fn setup_search_engine() -> (SearchEngine, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let search_engine = SearchEngine::new(temp_dir.path().to_str().unwrap())
        .expect("Failed to create search engine");
    (search_engine, temp_dir)
}

fn bench_document_indexing(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = setup_search_engine();

    let document = cms_backend::search::Document {
        id: Uuid::new_v4(),
        title: "Performance Optimization in Rust".to_string(),
        content: "Learn how to optimize your Rust applications for maximum performance. \
                  This comprehensive guide covers memory management, compiler optimizations, \
                  and profiling techniques.".to_string(),
        excerpt: Some("A guide to Rust performance optimization".to_string()),
        tags: vec!["rust".to_string(), "performance".to_string(), "optimization".to_string()],
        author: "benchmark_author".to_string(),
        created_at: chrono::Utc::now(),
    };

    c.bench_function("document_indexing", |b| {
        b.iter(|| {
            search_engine.index_document(black_box(&document))
        })
    });
}

fn bench_bulk_indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_indexing");
    
    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let (mut search_engine, _temp_dir) = setup_search_engine();
            
            let documents: Vec<_> = (0..size)
                .map(|i| cms_backend::search::Document {
                    id: Uuid::new_v4(),
                    title: format!("Document {} about Rust programming", i),
                    content: format!("Content for document {}. This covers various aspects of Rust.", i),
                    excerpt: Some(format!("Excerpt for document {}", i)),
                    tags: vec!["rust".to_string(), "programming".to_string()],
                    author: "benchmark_author".to_string(),
                    created_at: chrono::Utc::now(),
                })
                .collect();

            b.iter(|| {
                for doc in &documents {
                    let _ = search_engine.index_document(black_box(doc));
                }
            });
        });
    }
    
    group.finish();
}

fn bench_simple_search(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = setup_search_engine();
    
    // Index some documents first
    for i in 0..100 {
        let doc = cms_backend::search::Document {
            id: Uuid::new_v4(),
            title: format!("Rust Tutorial {}", i),
            content: format!("This is a comprehensive tutorial about Rust programming, covering topic {}", i),
            excerpt: Some(format!("Tutorial excerpt {}", i)),
            tags: vec!["rust".to_string(), "tutorial".to_string()],
            author: "benchmark_author".to_string(),
            created_at: chrono::Utc::now(),
        };
        search_engine.index_document(&doc).ok();
    }
    search_engine.commit().ok();

    let query = SearchQuery {
        query: "rust programming".to_string(),
        page: 1,
        per_page: 10,
    };

    c.bench_function("simple_search", |b| {
        b.iter(|| search_engine.search(black_box(&query)))
    });
}

fn bench_complex_search(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = setup_search_engine();
    
    // Index documents with varied content
    for i in 0..200 {
        let doc = cms_backend::search::Document {
            id: Uuid::new_v4(),
            title: format!("Article about {} and {}", 
                if i % 2 == 0 { "performance" } else { "security" },
                if i % 3 == 0 { "optimization" } else { "best practices" }
            ),
            content: format!("Detailed content about various aspects of programming. Index: {}", i),
            excerpt: Some(format!("Excerpt for article {}", i)),
            tags: vec![
                "rust".to_string(),
                if i % 2 == 0 { "performance" } else { "security" }.to_string(),
            ],
            author: format!("author_{}", i % 10),
            created_at: chrono::Utc::now(),
        };
        search_engine.index_document(&doc).ok();
    }
    search_engine.commit().ok();

    let query = SearchQuery {
        query: "performance optimization best practices".to_string(),
        page: 1,
        per_page: 20,
    };

    c.bench_function("complex_search", |b| {
        b.iter(|| search_engine.search(black_box(&query)))
    });
}

fn bench_search_pagination(c: &mut Criterion) {
    let (mut search_engine, _temp_dir) = setup_search_engine();
    
    // Index documents
    for i in 0..500 {
        let doc = cms_backend::search::Document {
            id: Uuid::new_v4(),
            title: format!("Document {}", i),
            content: format!("Content about Rust programming {}", i),
            excerpt: None,
            tags: vec!["rust".to_string()],
            author: "benchmark_author".to_string(),
            created_at: chrono::Utc::now(),
        };
        search_engine.index_document(&doc).ok();
    }
    search_engine.commit().ok();

    let mut group = c.benchmark_group("search_pagination");

    for page in [1, 5, 10, 20].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(page), page, |b, &page| {
            let query = SearchQuery {
                query: "rust programming".to_string(),
                page,
                per_page: 10,
            };

            b.iter(|| search_engine.search(black_box(&query)));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_document_indexing,
    bench_bulk_indexing,
    bench_simple_search,
    bench_complex_search,
    bench_search_pagination
);
criterion_main!(benches);
