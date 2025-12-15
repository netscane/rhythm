# MemtableContext Benchmark Suite

This directory contains comprehensive benchmarks for the `MemtableContext` implementation, which is a critical component for buffered repository operations.

## Benchmark Categories

### 1. Sequential Inserts (`memtable_sequential_inserts`)
Measures throughput for sequential insert operations with varying dataset sizes (100, 1000, 10000 items).

**What it tests:**
- Basic insert performance
- Memory allocation overhead
- Index building performance

### 2. Concurrent Inserts (`memtable_concurrent_inserts`)
Measures throughput for concurrent insert operations from multiple async tasks (2, 4, 8, 16 tasks).

**What it tests:**
- Lock contention under concurrent writes
- Scalability with multiple writers
- Thread safety overhead

### 3. Get Operations (`memtable_get_operations`)
Measures throughput for primary key lookups after pre-populating the memtable.

**What it tests:**
- Read performance
- HashMap lookup efficiency
- Lock contention on read operations

### 4. Index Lookups (`memtable_index_lookups`)
Measures throughput for secondary index lookups by name field.

**What it tests:**
- Secondary index performance
- Index lookup overhead vs primary key lookup
- String-based index efficiency

### 5. Mixed Read-Write Operations (`memtable_mixed_read_write`)
Measures throughput for workloads with 50% reads and 50% writes.

**What it tests:**
- Real-world mixed workload performance
- Lock contention in mixed scenarios
- Read-write fairness

### 6. Rotation and Flush (`memtable_rotation_flush`)
Measures throughput when memtable rotations are triggered by size thresholds.

**What it tests:**
- Rotation overhead
- Background flush performance
- Memory management during rotation
- Backpressure control with oneshot channels

### 7. Delete Operations (`memtable_delete_operations`)
Measures throughput for delete operations (tombstone marking).

**What it tests:**
- Tombstone insertion performance
- Index cleanup overhead
- Delete throughput

### 8. Concurrent Read-Heavy Workload (`memtable_concurrent_read_heavy`)
Simulates a read-heavy workload with 8 concurrent readers performing 1000 reads each on a pre-populated memtable of 10k items.

**What it tests:**
- Read scalability
- RwLock read contention
- Cache locality effects

### 9. Update Operations (`memtable_update_operations`)
Measures throughput for updating existing keys (insert with existing key).

**What it tests:**
- Update vs insert performance
- Index rebuilding overhead
- Old value cleanup performance

## Running Benchmarks

### Run All Benchmarks
```bash
cd src/crates/infra
cargo bench
```

### Run Specific Benchmark
```bash
# Run only sequential insert benchmarks
cargo bench --bench memtable_benchmark -- sequential_inserts

# Run only concurrent benchmarks
cargo bench --bench memtable_benchmark -- concurrent

# Run only rotation benchmarks
cargo bench --bench memtable_benchmark -- rotation
```

### Run with Custom Settings
```bash
# Run with sample size of 10
cargo bench --bench memtable_benchmark -- --sample-size 10

# Run with warm-up time of 5 seconds
cargo bench --bench memtable_benchmark -- --warm-up-time 5

# Save baseline for comparison
cargo bench --bench memtable_benchmark -- --save-baseline my-baseline

# Compare against baseline
cargo bench --bench memtable_benchmark -- --baseline my-baseline
```

## Output

Benchmark results are saved in:
- `target/criterion/` - Detailed HTML reports
- Console output - Summary statistics (mean, median, std dev)

Open the HTML report:
```bash
open target/criterion/report/index.html
```

## Interpreting Results

### Key Metrics
- **Throughput**: Operations per second (higher is better)
- **Time**: Average time per operation (lower is better)
- **Variance**: Consistency of performance (lower is better)

### Expected Performance Characteristics

1. **Sequential Inserts**: Should scale linearly with data size
2. **Concurrent Inserts**: Should show good scaling up to ~4-8 tasks, then plateau due to lock contention
3. **Gets**: Should be very fast (O(1) HashMap lookup) with minimal variance
4. **Index Lookups**: Slightly slower than primary key gets but still O(1)
5. **Rotation**: One-time overhead, should complete within milliseconds
6. **Read-Heavy**: Should scale well with concurrent readers (RwLock allows multiple readers)

## Performance Tuning

Based on benchmark results, you can tune:

1. **Threshold Size** (`active_threshold_size`): 
   - Smaller: More frequent rotations, lower memory usage
   - Larger: Less rotation overhead, higher memory usage

2. **Flush Timeout** (`flush_timeout`):
   - Shorter: More frequent flushes, fresher data in DB
   - Longer: Better batching, higher throughput

3. **Concurrency Level**:
   - Monitor the concurrent insert benchmarks to find optimal parallelism

## Benchmarking Best Practices

1. **Stable Environment**: Run benchmarks on a quiet system
2. **Consistent Conditions**: Close background applications
3. **Multiple Runs**: Average multiple runs for reliability
4. **Baseline Comparison**: Save baselines before making changes
5. **Profile Hot Paths**: Use `cargo flamegraph` for detailed profiling

## Example Output

```
memtable_sequential_inserts/100
                        time:   [45.234 µs 45.678 µs 46.123 µs]
                        thrpt:  [2.1682 Melem/s 2.1894 Melem/s 2.2108 Melem/s]

memtable_concurrent_inserts/4
                        time:   [1.2345 ms 1.2567 ms 1.2789 ms]
                        thrpt:  [3.1234 Melem/s 3.1789 Melem/s 3.2345 Melem/s]
```

## Contributing

When adding new benchmark scenarios:

1. Keep benchmarks focused on a single aspect
2. Use meaningful dataset sizes
3. Document what the benchmark tests
4. Ensure benchmarks are repeatable
5. Add to this README

## Troubleshooting

### Benchmark Takes Too Long
- Reduce sample size: `--sample-size 10`
- Reduce warm-up time: `--warm-up-time 1`
- Run specific benchmarks only

### Inconsistent Results
- Close background applications
- Disable CPU frequency scaling
- Run on a dedicated benchmark machine

### Out of Memory
- Reduce dataset sizes in the benchmark code
- Check for memory leaks in the implementation
