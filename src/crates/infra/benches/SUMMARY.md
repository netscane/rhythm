# MemtableContext Benchmark Suite - Summary

## Overview

This benchmark suite provides comprehensive performance testing for the `MemtableContext` implementation, a critical component in the buffered repository layer inspired by LSM-tree architecture.

## Files Created

### 1. `memtable_benchmark.rs`
The main benchmark file containing 9 comprehensive benchmark scenarios:

- **Sequential Inserts**: Tests baseline insert performance (100, 1K, 10K items)
- **Concurrent Inserts**: Tests multi-threaded scalability (2, 4, 8, 16 tasks)
- **Get Operations**: Tests primary key lookup performance
- **Index Lookups**: Tests secondary index query performance
- **Mixed Read-Write**: Tests realistic 50/50 read-write workloads
- **Rotation & Flush**: Tests memtable rotation and background flush
- **Delete Operations**: Tests tombstone marking performance
- **Concurrent Read-Heavy**: Tests read scalability with 8 concurrent readers
- **Update Operations**: Tests in-place update performance

**Key Features:**
- Uses Criterion.rs for statistical analysis
- Async/await support via Tokio runtime
- Mock persister (NoOpPersister) for isolated testing
- Realistic benchmark data structures
- Throughput measurements in elements/second
- Multiple dataset sizes for scaling analysis

### 2. `README.md`
Comprehensive guide explaining:
- What each benchmark tests
- How to run benchmarks (all, specific, filtered)
- How to interpret results
- Performance tuning guidelines
- Troubleshooting tips

### 3. `../BENCHMARKING.md`
Detailed benchmarking guide including:
- Quick start instructions
- Performance expectations and tuning
- Configuration parameter tradeoffs
- Profiling techniques (CPU, memory)
- CI/CD integration examples
- Best practices and workflows

### 4. `../bench.sh`
Convenient shell script for common tasks:
```bash
./bench.sh all           # Run all benchmarks
./bench.sh quick         # Quick benchmarks with reduced samples
./bench.sh sequential    # Only sequential benchmarks
./bench.sh concurrent    # Only concurrent benchmarks
./bench.sh rotation      # Only rotation benchmarks
./bench.sh baseline foo  # Save baseline
./bench.sh compare foo   # Compare with baseline
./bench.sh flamegraph    # Generate CPU flamegraph
./bench.sh report        # Open HTML report
```

## Quick Start

```bash
# Navigate to infra crate
cd src/crates/infra

# Run all benchmarks
cargo bench

# Or use the convenience script
./bench.sh all

# View results
open ../../target/criterion/report/index.html
```

## Benchmark Architecture

### Mock Components

**BenchmarkItem**: Test data structure with:
- Primary key: `id` (i64)
- Secondary indexes: `name`, `category`, `value`
- Realistic data generation

**NoOpPersister**: Mock persister that:
- Tracks persist/remove counts
- Has no I/O overhead
- Allows pure in-memory benchmarking

### Design Decisions

1. **Isolated Testing**: NoOpPersister removes database I/O from benchmarks
2. **Multiple Sizes**: Tests with 100, 1K, 10K items to reveal scaling characteristics
3. **Concurrent Scenarios**: Tests with 2, 4, 8, 16 tasks to measure lock contention
4. **Realistic Workloads**: Mixed read-write and read-heavy scenarios
5. **Statistical Rigor**: Uses Criterion for confidence intervals and variance analysis

## Performance Characteristics

### Expected Results

Based on the implementation:

| Operation | Expected Throughput | Complexity |
|-----------|---------------------|------------|
| Sequential Insert | 1-2M ops/sec | O(1) + index overhead |
| Concurrent Insert | 3-4M ops/sec (4 tasks) | Limited by lock contention |
| Get (Primary Key) | 5-10M ops/sec | O(1) HashMap lookup |
| Get (Index) | 3-5M ops/sec | O(1) + extra hop |
| Delete | 1-2M ops/sec | O(1) + tombstone |
| Update | 1-2M ops/sec | O(1) + index rebuild |

### Bottlenecks

1. **RwLock Contention**: Write operations require exclusive lock
2. **Index Maintenance**: Multiple indexes add overhead on insert/update
3. **Rotation Cost**: One-time cost when crossing threshold
4. **Memory Allocation**: Arc<V> allocations for each item

## Tuning Guidelines

### Threshold Size

| Value | Memory | Rotation Frequency | Use Case |
|-------|--------|-------------------|----------|
| 100-1K | Low | High | Memory-constrained, fast recovery |
| 10K-50K | Medium | Medium | Balanced |
| 100K+ | High | Low | High-throughput, batch-oriented |

### Flush Timeout

| Value | Data Freshness | I/O Frequency | Use Case |
|-------|----------------|---------------|----------|
| 5-10s | High | High | Near real-time requirements |
| 30-60s | Medium | Medium | Balanced |
| 300s+ | Low | Low | Batch processing |

## Integration with CI/CD

Example GitHub Actions workflow:

```yaml
name: Benchmark

on:
  pull_request:
    paths:
      - 'src/crates/infra/src/repository/buffered/**'

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Run benchmarks
        run: |
          cd src/crates/infra
          cargo bench --bench memtable_benchmark -- --save-baseline pr
      
      - name: Upload results
        uses: actions/upload-artifact@v2
        with:
          name: benchmark-results
          path: target/criterion/
```

## Future Enhancements

Potential additions:
1. **Memory profiling**: Track heap allocations
2. **Cache analysis**: Measure cache hit rates
3. **Stress testing**: Very large datasets (1M+ items)
4. **Latency percentiles**: P50, P95, P99 measurements
5. **Comparison with alternatives**: Benchmark against other memtable implementations
6. **Real persister**: Benchmark with actual database writes
7. **Network latency**: Simulate database network delays

## Contributing

When modifying `MemtableContext`:

1. Run baseline: `./bench.sh baseline before-change`
2. Make changes
3. Run comparison: `./bench.sh compare before-change`
4. Document performance impact in PR
5. Ensure no significant regressions (>5% slowdown)

## References

- **Criterion.rs**: https://bheisler.github.io/criterion.rs/
- **LSM Trees**: Log-Structured Merge Trees
- **RocksDB Memtable**: Similar implementation in C++
- **Rust Performance**: https://nnethercote.github.io/perf-book/

## Metrics Glossary

- **Throughput**: Operations per second (higher is better)
- **Latency**: Time per operation (lower is better)  
- **Variance**: Consistency of measurements (lower is better)
- **Confidence Interval**: Range of likely true values
- **p-value**: Statistical significance (< 0.05 = significant change)

## Contact

For questions or issues with benchmarks:
- Check existing benchmark results in `target/criterion/`
- Review documentation in `BENCHMARKING.md`
- Run `./bench.sh help` for usage
