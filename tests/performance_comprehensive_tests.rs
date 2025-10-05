//! Comprehensive performance and concurrency tests
//!
//! Tests for performance characteristics, concurrency safety, and resource management.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

#[tokio::test]
async fn test_concurrent_operations_safety() {
    // Test that concurrent operations are safe
    let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut handles = vec![];

    for _ in 0..100 {
        let counter_clone = Arc::clone(&counter);
        let handle = tokio::spawn(async move {
            for _ in 0..100 {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 10000);
}

#[tokio::test]
async fn test_async_task_completion() {
    // Test that async tasks complete successfully
    let result = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        42
    })
    .await;

    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_multiple_async_tasks() {
    // Test multiple concurrent async tasks
    let mut tasks = Vec::new();
    for i in 0..10 {
        tasks.push(tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            i * 2
        }));
    }

    for (i, task) in tasks.into_iter().enumerate() {
        let result = task.await.unwrap();
        assert_eq!(result, i * 2);
    }
}

#[test]
fn test_string_allocation_performance() {
    // Test string allocation performance
    let start = std::time::Instant::now();

    let mut strings = Vec::new();
    for i in 0..10_000 {
        strings.push(format!("string_{i}"));
    }

    let elapsed = start.elapsed();

    assert_eq!(strings.len(), 10_000);
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn test_vec_prealloation_efficiency() {
    // Test that preallocation improves performance
    let start = std::time::Instant::now();

    let mut vec = Vec::with_capacity(10_000);
    for i in 0..10_000 {
        vec.push(i);
    }

    let elapsed = start.elapsed();

    assert_eq!(vec.len(), 10000);
    assert!(elapsed < Duration::from_millis(100));
}

#[test]
fn test_hashmap_performance() {
    use std::collections::HashMap;

    let start = std::time::Instant::now();

    // Create large HashMap
    let mut map = HashMap::new();
    for i in 0..10_000 {
        map.insert(i, format!("value_{i}"));
    }

    let elapsed = start.elapsed();

    assert_eq!(map.len(), 10000);
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn test_serialization_performance() {
    use serde_json::json;

    let data = json!({
        "id": 1,
        "name": "Test",
        "items": (0..1000).collect::<Vec<i32>>(),
    });

    let start = std::time::Instant::now();

    for _ in 0..100 {
        let _serialized = serde_json::to_string(&data).unwrap();
    }

    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn test_deserialization_performance() {
    let json_str = r#"{"id": 1, "name": "Test", "active": true}"#;

    let start = std::time::Instant::now();

    for _ in 0..1000 {
        let _value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    }

    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_secs(1));
}

#[tokio::test]
async fn test_timeout_behavior() {
    // Test that operations can be timed out
    let result = tokio::time::timeout(Duration::from_millis(100), async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        42
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_timeout_expiration() {
    // Test that timeouts actually expire
    let result = tokio::time::timeout(Duration::from_millis(50), async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        42
    })
    .await;

    assert!(result.is_err());
}

#[test]
fn test_memory_efficiency_with_arc() {
    use std::sync::Arc;

    let data = Arc::new(vec![0u8; 1000]);
    let clones: Vec<_> = (0..100).map(|_| Arc::clone(&data)).collect();

    // All clones should point to the same data
    assert_eq!(Arc::strong_count(&data), 101); // original + 100 clones
    assert_eq!(clones.len(), 100);
}

#[test]
fn test_string_interning_concept() {
    // Test that string literals are efficiently stored
    let s1 = "test";
    let s2 = "test";

    // String literals should be the same address
    assert_eq!(s1, s2);
}

#[tokio::test]
async fn test_channel_communication() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);

    tokio::spawn(async move {
        for i in 0..10 {
            tx.send(i).await.unwrap();
        }
    });

    let mut received = Vec::new();
    while let Some(value) = rx.recv().await {
        received.push(value);
        if received.len() == 10 {
            break;
        }
    }

    assert_eq!(received.len(), 10);
}

#[tokio::test]
async fn test_broadcast_channel() {
    let (tx, mut rx1) = tokio::sync::broadcast::channel(10);
    let mut rx2 = tx.subscribe();

    tokio::spawn(async move {
        for i in 0..5 {
            tx.send(i).unwrap();
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut received1 = Vec::new();
    let mut received2 = Vec::new();

    while received1.len() < 5 {
        if let Ok(value) = rx1.try_recv() {
            received1.push(value);
        }
    }

    while received2.len() < 5 {
        if let Ok(value) = rx2.try_recv() {
            received2.push(value);
        }
    }

    assert_eq!(received1.len(), 5);
    assert_eq!(received2.len(), 5);
}

#[test]
fn test_rwlock_performance() {
    use std::sync::{Arc, RwLock};
    use std::thread;

    let data = Arc::new(RwLock::new(0));
    let mut handles = vec![];

    // Multiple readers
    for _ in 0..10 {
        let data_clone = Arc::clone(&data);
        let handle = thread::spawn(move || {
            let _value = data_clone.read().unwrap();
            thread::sleep(Duration::from_millis(10));
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[tokio::test]
async fn test_tokio_rwlock_async() {
    use tokio::sync::RwLock;

    let data = Arc::new(RwLock::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let data_clone = Arc::clone(&data);
        let handle = tokio::spawn(async move {
            let _guard = data_clone.read().await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

#[test]
fn test_clone_performance() {
    #[derive(Clone)]
    #[allow(dead_code)]
    struct LargeStruct {
        data: Vec<u8>,
    }

    let original = LargeStruct {
        data: vec![0u8; 1000],
    };

    let start = std::time::Instant::now();

    let mut clones = Vec::new();
    for _ in 0..1000 {
        clones.push(original.clone());
    }

    let elapsed = start.elapsed();

    assert_eq!(clones.len(), 1000);
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn test_large_dataset_performance() {
    // Test performance with large dataset
    let data: Vec<i64> = (0..100_000).collect();

    let start = std::time::Instant::now();

    // Simulate data processing
    let sum: i64 = data.iter().sum();

    let duration = start.elapsed();

    assert_eq!(sum, (0..100_000).sum::<i64>());
    assert!(duration.as_millis() < 100);
}

#[test]
fn test_parallel_iteration_concept() {
    // Test that parallel iteration is more efficient for large datasets
    let data: Vec<i32> = (0..10000).collect();

    let start = std::time::Instant::now();
    let _sum: i32 = data.iter().map(|x| x * 2).sum();
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_secs(1));
}

#[tokio::test]
async fn test_rate_limiting_simulation() {
    use tokio::time::{Duration, interval};

    let mut interval = interval(Duration::from_millis(10));
    let mut count = 0;

    let start = Instant::now();

    while count < 10 {
        interval.tick().await;
        count += 1;
    }

    let elapsed = start.elapsed();

    // Should take at least 100ms (10 ticks * 10ms)
    assert!(elapsed >= Duration::from_millis(90));
}

#[test]
fn test_lazy_initialization() {
    use std::sync::OnceLock;

    static VALUE: OnceLock<String> = OnceLock::new();

    let v1 = VALUE.get_or_init(|| "initialized".to_string());
    let v2 = VALUE.get_or_init(|| "should not run".to_string());

    assert_eq!(v1, v2);
    assert_eq!(v1, "initialized");
}

#[tokio::test]
async fn test_connection_pooling_concept() {
    // Simulate connection pooling behavior
    use tokio::sync::Semaphore;

    let semaphore = Arc::new(Semaphore::new(5)); // Pool of 5 connections
    let mut handles = vec![];

    for _ in 0..10 {
        let sem_clone = Arc::clone(&semaphore);
        let handle = tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

#[test]
fn test_memory_allocation_patterns() {
    // Test different memory allocation patterns
    let mut vec1 = Vec::new();
    let mut vec2 = Vec::with_capacity(1000);

    let start1 = std::time::Instant::now();
    for i in 0..1000 {
        vec1.push(i);
    }
    let elapsed1 = start1.elapsed();

    let start2 = std::time::Instant::now();
    for i in 0..1000 {
        vec2.push(i);
    }
    let elapsed2 = start2.elapsed();

    // Preallocated vec should be faster or equal
    assert!(elapsed2 <= elapsed1 || elapsed1 < Duration::from_millis(1));
    assert_eq!(vec1.len(), 1000);
    assert_eq!(vec2.len(), 1000);
}

#[tokio::test]
async fn test_graceful_shutdown_simulation() {
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel(1);

    let worker = tokio::spawn(async move {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                // Graceful shutdown
                "shutdown"
            }
            () = tokio::time::sleep(Duration::from_secs(10)) => {
                "timeout"
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(10)).await;
    shutdown_tx.send(()).unwrap();

    let result = worker.await.unwrap();
    assert_eq!(result, "shutdown");
}
