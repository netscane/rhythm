use async_trait::async_trait;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use infra::repository::buffered::memtable::{
    IndexValue, IndexMatch, Memtable, MemtableContext, MemtablePersister, MemtableValue,
};
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

// ============================================
// Mock Data Structures for Benchmarking
// ============================================

#[derive(Clone, Debug)]
struct BenchmarkItem {
    id: i64,
    name: String,
    category: String,
    value: i32,
}

impl BenchmarkItem {
    fn new(id: i64) -> Self {
        Self {
            id,
            name: format!("item_{}", id),
            category: format!("category_{}", id % 10),
            value: (id % 1000) as i32,
        }
    }
}

impl MemtableValue<i64> for BenchmarkItem {
    fn get_key(&self) -> i64 {
        self.id
    }

    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)> {
        vec![
            ("name", IndexValue::String(self.name.clone()), IndexMatch::Exact),
            ("category", IndexValue::String(self.category.clone()), IndexMatch::Exact),
            ("value", IndexValue::I32(self.value), IndexMatch::Exact),
        ]
    }

    fn get_index(&self, index_name: &str) -> IndexValue {
        match index_name {
            "name" => IndexValue::String(self.name.clone()),
            "category" => IndexValue::String(self.category.clone()),
            "value" => IndexValue::I32(self.value),
            _ => panic!("Invalid index name: {}", index_name),
        }
    }
}

// ============================================
// Mock Persister (No-op for pure benchmark)
// ============================================

#[derive(Clone)]
struct NoOpPersister {
    persist_count: Arc<AtomicUsize>,
    remove_count: Arc<AtomicUsize>,
}

impl NoOpPersister {
    fn new() -> Self {
        Self {
            persist_count: Arc::new(AtomicUsize::new(0)),
            remove_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl MemtablePersister<i64, BenchmarkItem> for NoOpPersister {
    async fn persist(&self, _key: i64, _value: Arc<BenchmarkItem>) -> Result<(), String> {
        self.persist_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    async fn remove(&self, _key: i64) -> Result<(), String> {
        self.remove_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    async fn persist_batch(&self, items: Vec<(i64, Arc<BenchmarkItem>)>) -> Result<(), String> {
        self.persist_count
            .fetch_add(items.len(), std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    async fn remove_batch(&self, keys: Vec<i64>) -> Result<(), String> {
        self.remove_count
            .fetch_add(keys.len(), std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

// ============================================
// Benchmark Helper Functions
// ============================================

fn create_memtable_context(
    threshold_size: usize,
    flush_timeout: Duration,
) -> Arc<MemtableContext<i64, BenchmarkItem, NoOpPersister>> {
    let memtable = Arc::new(RwLock::new(Memtable::<i64, BenchmarkItem>::new()));
    let size = Arc::new(AtomicUsize::new(0));
    let persister = Arc::new(NoOpPersister::new());

    Arc::new(MemtableContext::new(
        "benchmark".to_string(),
        memtable,
        size,
        threshold_size,
        persister,
        flush_timeout,
    ))
}

// ============================================
// Benchmark: Sequential Inserts
// ============================================

fn bench_sequential_inserts(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable_sequential_inserts");
    let rt = Runtime::new().unwrap();

    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let ctx = create_memtable_context(size * 2, Duration::from_secs(60));

                for i in 0..size {
                    let item = Arc::new(BenchmarkItem::new(i as i64));
                    ctx.insert(i as i64, item).await.unwrap();
                }

                black_box(ctx);
            });
        });
    }
    group.finish();
}

// ============================================
// Benchmark: Concurrent Inserts
// ============================================

fn bench_concurrent_inserts(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable_concurrent_inserts");
    let rt = Runtime::new().unwrap();

    for num_tasks in [2, 4, 8, 16] {
        let size_per_task = 1000;
        let total_size = num_tasks * size_per_task;

        group.throughput(Throughput::Elements(total_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_tasks),
            &num_tasks,
            |b, &num_tasks| {
                b.to_async(&rt).iter(|| async {
                    let ctx = create_memtable_context(total_size * 2, Duration::from_secs(60));

                    let mut handles = vec![];
                    for task_id in 0..num_tasks {
                        let ctx_clone = ctx.clone();
                        let handle = tokio::spawn(async move {
                            for i in 0..size_per_task {
                                let id = (task_id * size_per_task + i) as i64;
                                let item = Arc::new(BenchmarkItem::new(id));
                                ctx_clone.insert(id, item).await.unwrap();
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }

                    black_box(ctx);
                });
            },
        );
    }
    group.finish();
}

// ============================================
// Benchmark: Get Operations
// ============================================

fn bench_get_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable_get_operations");
    let rt = Runtime::new().unwrap();

    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let ctx = create_memtable_context(size * 2, Duration::from_secs(60));

                // Pre-populate
                for i in 0..size {
                    let item = Arc::new(BenchmarkItem::new(i as i64));
                    ctx.insert(i as i64, item).await.unwrap();
                }

                // Benchmark gets
                for i in 0..size {
                    black_box(ctx.get(&(i as i64)).await);
                }
            });
        });
    }
    group.finish();
}

// ============================================
// Benchmark: Index Lookups
// ============================================

fn bench_index_lookups(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable_index_lookups");
    let rt = Runtime::new().unwrap();

    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let ctx = create_memtable_context(size * 2, Duration::from_secs(60));

                // Pre-populate
                for i in 0..size {
                    let item = Arc::new(BenchmarkItem::new(i as i64));
                    ctx.insert(i as i64, item).await.unwrap();
                }

                // Benchmark index lookups by name
                for i in 0..size {
                    let name = format!("item_{}", i);
                    black_box(
                        ctx.get_by_index("name", IndexValue::String(name))
                            .await,
                    );
                }
            });
        });
    }
    group.finish();
}

// ============================================
// Benchmark: Mixed Read-Write Operations
// ============================================

fn bench_mixed_read_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable_mixed_read_write");
    let rt = Runtime::new().unwrap();

    for size in [100, 1000, 5000] {
        group.throughput(Throughput::Elements((size * 2) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let ctx = create_memtable_context(size * 2, Duration::from_secs(60));

                // Pre-populate half
                for i in 0..size / 2 {
                    let item = Arc::new(BenchmarkItem::new(i as i64));
                    ctx.insert(i as i64, item).await.unwrap();
                }

                // Mixed operations: 50% read, 50% write
                for i in 0..size {
                    if i % 2 == 0 {
                        // Read
                        black_box(ctx.get(&((i / 2) as i64)).await);
                    } else {
                        // Write
                        let item = Arc::new(BenchmarkItem::new(i as i64));
                        ctx.insert(i as i64, item).await.unwrap();
                    }
                }
            });
        });
    }
    group.finish();
}

// ============================================
// Benchmark: Rotation and Flush
// ============================================

fn bench_rotation_flush(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable_rotation_flush");
    let rt = Runtime::new().unwrap();

    for threshold in [100, 500, 1000] {
        group.throughput(Throughput::Elements((threshold * 2) as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(threshold),
            &threshold,
            |b, &threshold| {
                b.to_async(&rt).iter(|| async {
                    let ctx = create_memtable_context(threshold, Duration::from_secs(60));

                    // Insert enough to trigger rotation twice
                    for i in 0..(threshold * 2) {
                        let item = Arc::new(BenchmarkItem::new(i as i64));
                        ctx.insert(i as i64, item).await.unwrap();
                    }

                    black_box(ctx);
                });
            },
        );
    }
    group.finish();
}

// ============================================
// Benchmark: Delete Operations
// ============================================

fn bench_delete_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable_delete_operations");
    let rt = Runtime::new().unwrap();

    for size in [100, 1000, 5000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let ctx = create_memtable_context(size * 2, Duration::from_secs(60));

                // Pre-populate
                for i in 0..size {
                    let item = Arc::new(BenchmarkItem::new(i as i64));
                    ctx.insert(i as i64, item).await.unwrap();
                }

                // Benchmark deletes
                for i in 0..size {
                    ctx.delete(&(i as i64)).await.unwrap();
                }
            });
        });
    }
    group.finish();
}

// ============================================
// Benchmark: Concurrent Read-Heavy Workload
// ============================================

fn bench_concurrent_read_heavy(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable_concurrent_read_heavy");
    let rt = Runtime::new().unwrap();

    let size = 10000;
    let num_readers = 8;
    let reads_per_reader = 1000;

    group.throughput(Throughput::Elements(
        (num_readers * reads_per_reader) as u64,
    ));
    group.bench_function("8_readers_10k_items", |b| {
        b.to_async(&rt).iter(|| async {
            let ctx = create_memtable_context(size * 2, Duration::from_secs(60));

            // Pre-populate
            for i in 0..size {
                let item = Arc::new(BenchmarkItem::new(i as i64));
                ctx.insert(i as i64, item).await.unwrap();
            }

            // Spawn readers
            let mut handles = vec![];
            for _ in 0..num_readers {
                let ctx_clone = ctx.clone();
                let handle = tokio::spawn(async move {
                    for i in 0..reads_per_reader {
                        let id = (i % size) as i64;
                        black_box(ctx_clone.get(&id).await);
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.await.unwrap();
            }

            black_box(ctx);
        });
    });
    group.finish();
}

// ============================================
// Benchmark: Update Operations (Insert Existing Key)
// ============================================

fn bench_update_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable_update_operations");
    let rt = Runtime::new().unwrap();

    for size in [100, 1000, 5000] {
        group.throughput(Throughput::Elements((size * 2) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let ctx = create_memtable_context(size * 2, Duration::from_secs(60));

                // Initial insert
                for i in 0..size {
                    let item = Arc::new(BenchmarkItem::new(i as i64));
                    ctx.insert(i as i64, item).await.unwrap();
                }

                // Update (overwrite) existing keys
                for i in 0..size {
                    let mut item = BenchmarkItem::new(i as i64);
                    item.value = (i * 2) as i32; // Different value
                    ctx.insert(i as i64, Arc::new(item)).await.unwrap();
                }

                black_box(ctx);
            });
        });
    }
    group.finish();
}

// ============================================
// Criterion Configuration
// ============================================

criterion_group!(
    benches,
    bench_sequential_inserts,
    bench_concurrent_inserts,
    bench_get_operations,
    bench_index_lookups,
    bench_mixed_read_write,
    bench_rotation_flush,
    bench_delete_operations,
    bench_concurrent_read_heavy,
    bench_update_operations,
);

criterion_main!(benches);
