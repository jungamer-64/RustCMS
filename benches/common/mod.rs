//! Shared utilities for benchmark suite
//!
//! This module provides common functionality for all benchmarks including:
//! - Test data generation
//! - Setup helpers
//! - Metrics collection
//! - Result formatting
//! - Configuration management
//! - Error handling utilities

use std::env;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use uuid::Uuid;

// ============================================================================
// Configuration Management
// ============================================================================

/// Benchmark configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub biscuit_root_key_path: String,
    pub biscuit_public_key_path: String,
    pub max_concurrency: usize,
    pub enable_memory_profiling: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            biscuit_root_key_path: env::var("BISCUIT_ROOT_KEY_PATH")
                .unwrap_or_else(|_| "./biscuit_keys/root.key".to_string()),
            biscuit_public_key_path: env::var("BISCUIT_PUBLIC_KEY_PATH")
                .unwrap_or_else(|_| "./biscuit_keys/public.key".to_string()),
            max_concurrency: env::var("BENCH_MAX_CONCURRENCY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| num_cpus::get() * 2),
            enable_memory_profiling: env::var("BENCH_MEMORY_PROFILE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(false),
        }
    }
}

static CONFIG: OnceLock<BenchmarkConfig> = OnceLock::new();

/// Get global benchmark configuration
pub fn config() -> &'static BenchmarkConfig {
    CONFIG.get_or_init(BenchmarkConfig::default)
}

/// Get dynamic concurrency levels based on CPU count
pub fn concurrency_levels() -> Vec<usize> {
    let max = config().max_concurrency;
    let cpu_count = num_cpus::get();

    vec![
        1,
        cpu_count / 2,
        cpu_count,
        cpu_count * 2,
        max.min(cpu_count * 4),
    ]
    .into_iter()
    .filter(|&x| x > 0 && x <= max)
    .collect()
}

// ============================================================================
// Error Handling Utilities
// ============================================================================

/// Result type for benchmark operations
pub type BenchResult<T> = Result<T, BenchError>;

/// Benchmark error types
#[derive(Debug)]
pub enum BenchError {
    Setup(String),
    Execution(String),
    Teardown(String),
}

impl std::fmt::Display for BenchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BenchError::Setup(msg) => write!(f, "Benchmark setup error: {}", msg),
            BenchError::Execution(msg) => write!(f, "Benchmark execution error: {}", msg),
            BenchError::Teardown(msg) => write!(f, "Benchmark teardown error: {}", msg),
        }
    }
}

impl std::error::Error for BenchError {}

/// Helper to handle benchmark errors with context
pub fn handle_bench_error<T, E: std::fmt::Display>(
    result: Result<T, E>,
    context: &str,
) -> BenchResult<T> {
    result.map_err(|e| BenchError::Execution(format!("{}: {}", context, e)))
}

// ============================================================================
// Test Data Generation (existing functions)
// ============================================================================

/// Generate a unique identifier for test data
pub fn generate_test_id() -> Uuid {
    Uuid::new_v4()
}

/// Generate test username with prefix
pub fn generate_test_username(prefix: &str, id: usize) -> String {
    format!("{}_{}", prefix, id)
}

/// Generate test email
pub fn generate_test_email(username: &str) -> String {
    format!("{}@benchmark.test", username)
}

/// Generate test content
pub fn generate_test_content(size: usize) -> String {
    let base = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";
    base.repeat(size / base.len() + 1)[..size].to_string()
}

/// Generate test tags
pub fn generate_test_tags(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("tag_{}", i)).collect()
}

/// Benchmark result holder
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration: Duration,
    pub iterations: u64,
    pub ops_per_sec: f64,
}

impl BenchmarkResult {
    pub fn new(name: String, duration: Duration, iterations: u64) -> Self {
        let ops_per_sec = iterations as f64 / duration.as_secs_f64();
        Self {
            name,
            duration,
            iterations,
            ops_per_sec,
        }
    }

    pub fn format_report(&self) -> String {
        format!(
            "{}: {} iterations in {:?} ({:.2} ops/sec)",
            self.name, self.iterations, self.duration, self.ops_per_sec
        )
    }
}

/// Timer for measuring benchmark execution
pub struct BenchmarkTimer {
    start: Instant,
}

impl BenchmarkTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn finish(self, name: String, iterations: u64) -> BenchmarkResult {
        BenchmarkResult::new(name, self.elapsed(), iterations)
    }
}

/// Statistics calculator for benchmark results
pub struct BenchmarkStats {
    values: Vec<f64>,
}

impl BenchmarkStats {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn add(&mut self, value: f64) {
        self.values.push(value);
    }

    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    pub fn median(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }

    pub fn std_dev(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let variance = self.values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
            / (self.values.len() - 1) as f64;
        variance.sqrt()
    }

    pub fn min(&self) -> f64 {
        self.values
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }

    pub fn max(&self) -> f64 {
        self.values
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }
}

impl Default for BenchmarkStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_test_id() {
        let id1 = generate_test_id();
        let id2 = generate_test_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_test_username() {
        let username = generate_test_username("bench", 42);
        assert_eq!(username, "bench_42");
    }

    #[test]
    fn test_generate_test_email() {
        let email = generate_test_email("testuser");
        assert_eq!(email, "testuser@benchmark.test");
    }

    #[test]
    fn test_generate_test_content() {
        let content = generate_test_content(100);
        assert_eq!(content.len(), 100);
    }

    #[test]
    fn test_generate_test_tags() {
        let tags = generate_test_tags(5);
        assert_eq!(tags.len(), 5);
        assert_eq!(tags[0], "tag_0");
        assert_eq!(tags[4], "tag_4");
    }

    #[test]
    fn test_benchmark_result() {
        let result = BenchmarkResult::new("test".to_string(), Duration::from_secs(1), 1000);
        assert_eq!(result.name, "test");
        assert_eq!(result.iterations, 1000);
        assert_eq!(result.ops_per_sec, 1000.0);
    }

    #[test]
    fn test_benchmark_timer() {
        let timer = BenchmarkTimer::start();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.elapsed();
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_benchmark_stats_mean() {
        let mut stats = BenchmarkStats::new();
        stats.add(1.0);
        stats.add(2.0);
        stats.add(3.0);
        assert_eq!(stats.mean(), 2.0);
    }

    #[test]
    fn test_benchmark_stats_median() {
        let mut stats = BenchmarkStats::new();
        stats.add(1.0);
        stats.add(2.0);
        stats.add(3.0);
        assert_eq!(stats.median(), 2.0);
    }

    #[test]
    fn test_benchmark_stats_std_dev() {
        let mut stats = BenchmarkStats::new();
        stats.add(1.0);
        stats.add(2.0);
        stats.add(3.0);
        let std_dev = stats.std_dev();
        assert!(std_dev > 0.0);
    }

    #[test]
    fn test_benchmark_stats_min_max() {
        let mut stats = BenchmarkStats::new();
        stats.add(1.0);
        stats.add(5.0);
        stats.add(3.0);
        assert_eq!(stats.min(), 1.0);
        assert_eq!(stats.max(), 5.0);
    }

    #[test]
    fn test_config_defaults() {
        let config = BenchmarkConfig::default();
        assert!(config.max_concurrency > 0);
        assert!(!config.biscuit_root_key_path.is_empty());
    }

    #[test]
    fn test_concurrency_levels() {
        let levels = concurrency_levels();
        assert!(!levels.is_empty());
        assert!(levels[0] >= 1);
        // Levels should be sorted
        for i in 1..levels.len() {
            assert!(levels[i] > levels[i - 1]);
        }
    }

    #[test]
    fn test_bench_error_display() {
        let error = BenchError::Setup("test error".to_string());
        let msg = format!("{}", error);
        assert!(msg.contains("setup"));
        assert!(msg.contains("test error"));
    }
}
