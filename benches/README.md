# RustCMS Backend Benchmark Suite

## Overview

This comprehensive benchmark suite measures the performance of critical components in the RustCMS backend system. The benchmarks are designed to provide actionable insights for performance optimization and regression detection.

## Benchmark Categories

### 1. Authentication Benchmarks (`auth_benchmark.rs`)

**Purpose**: Measure authentication system performance including token generation, verification, and password hashing.

**Key Benchmarks**:

- Token generation (access, refresh, Biscuit)
- Token verification by role
- Password hashing with Argon2 (multiple configurations)
- Concurrent token operations
- Mixed authentication workload

**Performance Targets**:

- Token generation: < 10ms
- Token verification: < 5ms
- Password hashing: 100-500ms (security vs performance)
- Concurrent ops: Linear scaling up to CPU cores

**Run Commands**:

```bash
# Run all auth benchmarks
cargo bench --bench auth_benchmark

# Run specific benchmark
cargo bench --bench auth_benchmark -- token_generation

# Run with specific concurrency level
cargo bench --bench auth_benchmark -- concurrent_token_generation
```

### 2. Cache Benchmarks (`cache_benchmark.rs`)

**Purpose**: Evaluate cache system performance across various access patterns and workloads.

**Key Benchmarks**:

- Single and bulk set/get operations
- Cache hit/miss performance
- TTL operations
- Eviction strategies
- Concurrent access (reads, writes, mixed)
- Different workload ratios (read-heavy, write-heavy)

**Performance Targets**:

- Single set/get: < 1ms
- Bulk operations: Linear scaling
- Concurrent access: Minimal lock contention
- Cache hit rate: > 90% in production scenarios

**Run Commands**:

```bash
# Run all cache benchmarks
cargo bench --bench cache_benchmark

# Run specific category
cargo bench --bench cache_benchmark -- basic_ops
cargo bench --bench cache_benchmark -- concurrent
cargo bench --bench cache_benchmark -- mixed_workload
```

### 3. Search Benchmarks (`search_benchmark.rs`)

**Purpose**: Measure full-text search engine performance using Tantivy.

**Key Benchmarks**:

- Document indexing (single, bulk, varying sizes)
- Search queries (simple, complex, varied patterns)
- Pagination performance
- Index maintenance (commit, update)
- Mixed indexing and search workload

**Performance Targets**:

- Single document indexing: < 10ms
- Bulk indexing (100 docs): < 500ms
- Simple search: < 50ms
- Complex search: < 200ms
- Pagination: O(1) time complexity

**Run Commands**:

```bash
# Run all search benchmarks
cargo bench --bench search_benchmark

# Run specific category
cargo bench --bench search_benchmark -- indexing
cargo bench --bench search_benchmark -- search
cargo bench --bench search_benchmark -- pagination
```

### 4. Database Benchmarks (`database_benchmark.rs`)

**Purpose**: Evaluate database operation performance and connection pool efficiency.

**Key Benchmarks**:

- Connection pool acquisition
- CRUD operations (Create, Read, Update, Delete)
- Query complexity (simple, joins, aggregates)
- Transaction handling (commit, rollback)
- Concurrent database access
- Pagination queries

**Performance Targets**:

- Connection acquisition: < 10ms
- Simple query: < 5ms
- Complex join: < 50ms
- Transaction commit: < 20ms
- Concurrent operations: Linear scaling

**Note**: Database benchmarks use mock implementations by default. For production benchmarking, configure a test database.

**Run Commands**:

```bash
# Run all database benchmarks
cargo bench --bench database_benchmark

# Run specific category
cargo bench --bench database_benchmark -- connection
cargo bench --bench database_benchmark -- query
cargo bench --bench database_benchmark -- crud
```

## Benchmark Analysis Tool

### Overview

The benchmark analyzer is a Rust-based tool that provides comprehensive analysis of benchmark results, including:

- Performance regression detection
- Statistical significance analysis
- Multi-format report generation (Markdown, CSV, HTML)
- Baseline comparison
- Performance categorization (excellent, good, acceptable, needs optimization, critical)

### Building the Analyzer

```bash
# Build the benchmark analyzer
cargo build --release --bin benchmark-analyzer

# The binary will be available at:
# target/release/benchmark-analyzer
```

### Usage

**Basic Analysis**:

```bash
# Analyze single benchmark result
./target/release/benchmark-analyzer auth-results.json

# Compare with baseline
./target/release/benchmark-analyzer auth-results.json baseline/auth-results.json
```

**Output Formats**:

The analyzer generates reports in multiple formats:

1. **Markdown Report** (`benchmark-analysis.md`):
   - Human-readable summary
   - Performance categories
   - Regression/improvement highlights
   - Detailed comparison tables

2. **CSV Report** (`benchmark-comparison.csv`):
   - Machine-readable data
   - Easy import into spreadsheets
   - Statistical analysis

3. **HTML Report** (`benchmark-report.html`):
   - Visual presentation
   - Color-coded performance indicators
   - Interactive tables

**Example Output**:

```markdown
# Benchmark Analysis Report

## Summary
- Total benchmarks: 15
- Regressions: 2 (13.3%)
- Improvements: 5 (33.3%)
- Unchanged: 8 (53.3%)

## Performance Categories
- Excellent (<10ms): 8 benchmarks
- Good (10-50ms): 5 benchmarks
- Needs Optimization (>100ms): 2 benchmarks

## Notable Changes
⚠️  token_generation: 8.30ms → 12.45ms (+49.9% regression, HIGH significance)
✅ password_hashing: 250.00ms → 200.00ms (-20.0% improvement, HIGH significance)
```

### Integration with CI/CD

The analyzer is automatically integrated into the GitHub Actions workflow:

```yaml
- name: Analyze benchmark results (Rust tool)
  run: |
    cargo build --release --bin benchmark-analyzer
    ./target/release/benchmark-analyzer results.json baseline.json
```

Reports are uploaded as GitHub Actions artifacts and available for download.

## Running Benchmarks

### Prerequisites

```bash
# Install Rust and Cargo (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Ensure all dependencies are installed
cargo build --release
```

### Run All Benchmarks

```bash
# Run entire benchmark suite
cargo bench

# Run with verbose output
cargo bench -- --verbose

# Save baseline for comparison
cargo bench -- --save-baseline main
```

### Run Specific Benchmarks

```bash
# By benchmark file
cargo bench --bench auth_benchmark
cargo bench --bench cache_benchmark
cargo bench --bench search_benchmark
cargo bench --bench database_benchmark

# By benchmark group
cargo bench -- auth/token
cargo bench -- cache/concurrent
cargo bench -- search/indexing

# By specific test
cargo bench -- "token_generation"
```

### Compare Performance

```bash
# Create baseline
cargo bench -- --save-baseline before-optimization

# Make changes...

# Compare against baseline
cargo bench -- --baseline before-optimization
```

## Interpreting Results

### Understanding Criterion Output

```
auth/token_generation   time:   [8.2145 ms 8.3012 ms 8.4129 ms]
                        change: [-5.2389% -3.8234% -2.1043%] (p = 0.00 < 0.05)
                        Performance has improved.
```

- **time**: Mean execution time with confidence interval
- **change**: Performance change from baseline (negative = improvement)
- **p value**: Statistical significance (< 0.05 = significant)

### Performance Categories

- **Excellent**: Within or exceeding performance targets
- **Good**: 10-20% slower than targets
- **Needs Optimization**: 20-50% slower than targets
- **Critical**: > 50% slower than targets

## Environment Configuration

### Environment Variables

Configure benchmarks using environment variables:

```bash
# Authentication keys path
export BISCUIT_ROOT_KEY_PATH="./biscuit_keys/root.key"
export BISCUIT_PUBLIC_KEY_PATH="./biscuit_keys/public.key"

# Concurrency settings (defaults to CPU cores * 2)
export BENCH_MAX_CONCURRENCY=16

# Enable memory profiling (experimental)
export BENCH_MEMORY_PROFILE=false
```

### Dynamic Concurrency

Benchmarks automatically adapt to your system's CPU count. Concurrent operations test at:

- 1 thread
- CPU_COUNT / 2
- CPU_COUNT
- CPU_COUNT * 2
- Min(CPU_COUNT * 4, MAX_CONCURRENCY)

## Benchmark Configuration

### Criterion Settings

All benchmarks use Criterion's default configuration:

- Warm-up time: 3 seconds
- Measurement time: 5 seconds
- Sample size: 100 iterations
- Confidence level: 95%

### Custom Configuration

To modify benchmark settings, edit the `criterion_group!` configuration:

```rust
criterion_group!(
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(10))
        .sample_size(200);
    targets = bench_function1, bench_function2
);
```

## Continuous Integration

### CI Benchmark Pipeline

Benchmarks should run on:

- Pull requests (against main branch baseline)
- Main branch merges (update baseline)
- Nightly builds (performance regression detection)

### GitHub Actions Setup

A complete CI workflow is provided in `.github/workflows/benchmarks.yml`. It includes:

**Automated Features:**

- Automated benchmark runs on PRs and main branch
- Baseline comparison against main branch
- Performance regression detection with warnings
- Artifact storage for historical analysis (30 days for PRs, 90 days for baselines)
- Automatic PR comments with performance summary
- Daily scheduled runs at 2 AM UTC
- Manual workflow dispatch support

**Benchmark Analysis:**

- Automatic performance categorization:
  - ✅ Excellent: < 1ms
  - ✓ Good: 1-100ms
  - ⚠️ Needs optimization: > 100ms
- JSON format results for programmatic analysis
- Combined text reports for easy review

**Comparison Job:**

- Separate job for baseline comparison
- Detailed comparison reports
- Historical trend analysis support

To enable, ensure the workflow file exists and is committed to your repository.

## Performance Optimization Guide

### Identifying Bottlenecks

1. **Run full benchmark suite**

   ```bash
   cargo bench 2>&1 | tee benchmark-results.txt
   ```

2. **Analyze results**
   - Look for operations significantly slower than targets
   - Check for non-linear scaling in concurrent benchmarks
   - Identify high variance (wide confidence intervals)

3. **Profile hot paths**

   ```bash
   cargo flamegraph --bench auth_benchmark
   ```

### Common Optimizations

#### For Authentication

- Use connection pooling for Biscuit key access
- Cache frequently verified tokens
- Batch token generation when possible

#### For Cache

- Adjust eviction policy based on hit rate
- Use appropriate TTL values
- Consider multi-tier caching (memory + Redis)

#### For Search

- Batch document indexing
- Optimize index commit frequency
- Use appropriate index settings for your use case

#### For Database

- Increase connection pool size for high concurrency
- Use prepared statements
- Optimize query indexes
- Batch operations when possible

## Troubleshooting

### Benchmark Failures

**Issue**: Benchmarks fail to compile

```bash
# Check feature flags
cargo check --all-features

# Verify dependencies
cargo update
```

**Issue**: Inconsistent results

```bash
# Run on isolated system
# Close other applications
# Increase sample size
cargo bench -- --sample-size 500
```

**Issue**: Out of memory

```bash
# Reduce concurrent operations
# Limit document corpus size
# Increase system limits
ulimit -n 4096
```

### Performance Regressions

If benchmarks show performance regression:

1. **Verify environment**
   - Same hardware/VM configuration
   - No background processes
   - Consistent system load

2. **Check code changes**

   ```bash
   git diff main...HEAD -- benches/
   ```

3. **Bisect to find culprit**

   ```bash
   git bisect start
   git bisect bad HEAD
   git bisect good main
   # Run benchmarks at each step
   ```

## CI/CD Integration Examples

### Viewing Benchmark Results in GitHub Actions

1. **From Pull Requests:**
   - Check the automated comment on your PR
   - View detailed results in the workflow artifacts
   - Look for ⚠️ warnings indicating potential regressions

2. **From Main Branch:**
   - Baseline results are saved for 90 days
   - Access via Actions tab → Workflow runs → Artifacts

3. **Scheduled Runs:**
   - Daily nightly reports available for 365 days
   - Track long-term performance trends

### Local Benchmark Workflow

```bash
# 1. Set environment variables
export BENCH_MAX_CONCURRENCY=8
export BISCUIT_ROOT_KEY_PATH="./keys/root.key"

# 2. Run benchmarks and save baseline
cargo bench -- --save-baseline my-feature

# 3. Make changes to code

# 4. Run benchmarks and compare
cargo bench -- --baseline my-feature

# 5. Review Criterion output for regressions
```

## Best Practices

### Writing New Benchmarks

1. **Use `black_box`** to prevent compiler optimizations

   ```rust
   b.iter(|| function(black_box(input)))
   ```

2. **Group related benchmarks**

   ```rust
   let mut group = c.benchmark_group("category");
   group.bench_function("test1", ...);
   group.bench_function("test2", ...);
   group.finish();
   ```

3. **Parametrize benchmarks** for different inputs

   ```rust
   for size in [10, 100, 1000].iter() {
       group.bench_with_input(BenchmarkId::from_parameter(size), ...);
   }
   ```

4. **Clean up resources** between runs

   ```rust
   b.iter_batched(|| setup(), |s| test(s), BatchSize::SmallInput);
   ```

5. **Use dynamic concurrency** from common module

   ```rust
   use common::concurrency_levels;
   
   for concurrency in concurrency_levels().iter() {
       // Your concurrent benchmark
   }
   ```

### Benchmark Hygiene

- Run benchmarks on dedicated hardware when possible
- Disable CPU frequency scaling
- Close unnecessary applications
- Use consistent compiler flags
- Document any environmental requirements

## Benchmark Testing

### Overview

The `tests/benchmark_tests.rs` file contains comprehensive unit tests to verify benchmark functionality and correctness. These tests ensure that benchmarks execute without errors, produce reasonable results, handle edge cases properly, and scale as expected.

### Test Categories

1. **Authentication Tests** (`auth_tests`)
   - Token generation performance
   - Token verification performance
   - Password hashing performance
   - Concurrent authentication operations

2. **Cache Tests** (`cache_tests`)
   - Cache set/get performance
   - Concurrent access patterns
   - Eviction performance

3. **Search Tests** (`search_tests`)
   - Document indexing performance
   - Search query performance
   - Bulk indexing performance
   - Pagination performance

4. **Database Tests** (`database_tests`)
   - Connection pool performance
   - Query performance
   - Transaction performance
   - Concurrent queries

5. **Integration Tests** (`integration_tests`)
   - End-to-end performance
   - Cache-backed queries
   - Search with caching

6. **Regression Tests** (`regression_tests`)
   - Memory leak detection
   - Linear scaling verification
   - Constant-time lookup verification

7. **Stress Tests** (`stress_tests`)
   - High concurrency (100 threads)
   - Large data processing (100k items)
   - Sustained load testing (5 seconds)

### Running Benchmark Tests

```bash
# Run all benchmark tests
cargo test --test benchmark_tests

# Run specific test category
cargo test --test benchmark_tests -- auth_tests
cargo test --test benchmark_tests -- cache_tests

# Run with output
cargo test --test benchmark_tests -- --nocapture

# Run ignored tests (long-running)
cargo test --test benchmark_tests -- --ignored
```

### Test Utilities

The test suite provides helper functions:

- `measure_time<F, R>(f: F) -> (R, Duration)`: Measures execution time of a function
- `assert_duration_in_range(duration, min, max)`: Asserts duration is within expected range

### Integration with Benchmarks

These tests complement the Criterion benchmarks by:
- Verifying correctness before performance measurement
- Ensuring benchmarks don't have logic errors
- Testing edge cases and stress conditions
- Validating scaling characteristics

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Flamegraph Guide](https://github.com/flamegraph-rs/flamegraph)

## Contributing

When adding new benchmarks:

1. Follow existing naming conventions
2. Add documentation comments
3. Include performance targets
4. Update this documentation
5. Run full suite before submitting PR

## License

Same as RustCMS Backend project.
