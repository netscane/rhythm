use async_trait::async_trait;
use domain::value::AlbumId;
use log::info;
use model::album_stats::{AlbumStats, AlbumStatsAdjustment, AlbumStatsRepository};
use model::ModelError;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::super::memtable::{
    IndexValue, IndexMatch, Memtable, MemtableContext, MemtablePersister, MemtableValue,
};

#[derive(Clone)]
struct AlbumStatsWrapper(AlbumStats);

impl MemtableValue<i64> for AlbumStatsWrapper {
    fn get_key(&self) -> i64 {
        self.0.album_id.as_i64()
    }

    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)> {
        vec![]
    }

    fn get_index(&self, _index_name: &str) -> IndexValue {
        panic!("No indexes defined for AlbumStats")
    }
}

pub struct AlbumStatsPersister<R>
where
    R: AlbumStatsRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
}

impl<R> Clone for AlbumStatsPersister<R>
where
    R: AlbumStatsRepository + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<R> AlbumStatsPersister<R>
where
    R: AlbumStatsRepository + Send + Sync + 'static,
{
    pub fn new(inner: Arc<R>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<R> MemtablePersister<i64, AlbumStatsWrapper> for AlbumStatsPersister<R>
where
    R: AlbumStatsRepository + Send + Sync + 'static,
{
    async fn persist(&self, _key: i64, value: Arc<AlbumStatsWrapper>) -> Result<(), String> {
        self.inner
            .save(value.0.clone())
            .await
            .map_err(|e| format!("Failed to save: {}", e))?;
        Ok(())
    }

    async fn remove(&self, key: i64) -> Result<(), String> {
        let album_id = AlbumId::from(key);
        self.inner
            .delete_by_album_id(album_id)
            .await
            .map_err(|e| format!("Failed to delete: {}", e))?;
        Ok(())
    }
}

pub struct BufferedAlbumStatsRepository<R>
where
    R: AlbumStatsRepository + Send + Sync + 'static,
{
    memtable_context: Arc<MemtableContext<i64, AlbumStatsWrapper, AlbumStatsPersister<R>>>,
    inner: Arc<R>,
}

impl<R> BufferedAlbumStatsRepository<R>
where
    R: AlbumStatsRepository + Send + Sync + 'static,
{
    pub fn new(inner: R, cache_capacity: usize, flush_timeout: Duration) -> Arc<Self> {
        let memtable_size_threshold = cache_capacity.max(100);

        // 创建 persister
        let inner_arc = Arc::new(inner);
        let persister = Arc::new(AlbumStatsPersister::new(inner_arc.clone()));

        // 创建 memtable 和相关组件
        let active_memtable = Arc::new(RwLock::new(Memtable::<i64, AlbumStatsWrapper>::new()));
        let active_size = Arc::new(AtomicUsize::new(0));

        let memtable_context = Arc::new(MemtableContext::new(
            "AlbumStats".to_string(),
            active_memtable.clone(),
            active_size.clone(),
            memtable_size_threshold,
            persister,
            flush_timeout,
        ));

        // 启动自动 flush 定时器
        memtable_context.start_auto_flush_timer();

        Arc::new(Self {
            memtable_context,
            inner: inner_arc,
        })
    }

    pub async fn shutdown_gracefully(
        &self,
        wait_duration: Duration,
    ) -> Result<Option<usize>, String> {
        info!("Starting graceful shutdown of BufferedAlbumStatsRepository");

        let flushed_count = self.memtable_context.shutdown_gracefully().await;

        if flushed_count.is_none() {
            info!("No data to flush during shutdown");
            return Ok(None);
        }

        let count = flushed_count.unwrap();
        info!(
            "Triggered flush of {} items, waiting for async tasks to complete",
            count
        );

        tokio::time::sleep(wait_duration).await;

        info!(
            "Graceful shutdown completed (waited {:?} for flush tasks)",
            wait_duration
        );
        Ok(Some(count))
    }
}

#[async_trait]
impl<R> AlbumStatsRepository for BufferedAlbumStatsRepository<R>
where
    R: AlbumStatsRepository + Send + Sync + 'static,
{
    async fn adjust_stats(&self, adjustment: AlbumStatsAdjustment) -> Result<(), ModelError> {
        let id_i64 = adjustment.album_id.as_i64();

        // 使用 update_or_insert 保证原子性，避免竞态条件
        self.memtable_context
            .update_or_insert(id_i64, |current| {
                let new_stats = match current {
                    Some(existing) => {
                        let mut stats = existing.0.clone();

                        // Apply deltas
                        stats.duration += adjustment.duration_delta;
                        stats.size += adjustment.size_delta;
                        stats.song_count += adjustment.song_count_delta;

                        // Add disk_number if provided and not already in the list
                        if let Some(disk_num) = adjustment.disk_number {
                            if !stats.disk_numbers.contains(&disk_num) {
                                stats.disk_numbers.push(disk_num);
                            }
                        }

                        // Set year if provided and not already set
                        if let Some(year_val) = adjustment.year {
                            if stats.year.is_none() || stats.year == Some(0) {
                                stats.year = Some(year_val);
                            }
                        }

                        stats
                    }
                    None => {
                        // Create new stats from adjustment
                        AlbumStats {
                            album_id: adjustment.album_id.clone(),
                            duration: adjustment.duration_delta.max(0),
                            size: adjustment.size_delta.max(0),
                            song_count: adjustment.song_count_delta.max(0),
                            disk_numbers: if let Some(disk_num) = adjustment.disk_number {
                                vec![disk_num]
                            } else {
                                vec![]
                            },
                            year: adjustment.year,
                        }
                    }
                };

                Arc::new(AlbumStatsWrapper(new_stats))
            })
            .await
            .map_err(|e| ModelError::DbErr(format!("Failed to update memtable: {}", e)))?;

        Ok(())
    }

    async fn find_by_album_id(&self, album_id: AlbumId) -> Result<Option<AlbumStats>, ModelError> {
        let id_i64 = album_id.as_i64();

        // 1. 先查 memtable
        if let Some(wrapper) = self.memtable_context.get(&id_i64).await {
            return Ok(Some(wrapper.0.clone()));
        }

        // 2. 查数据库
        self.inner.find_by_album_id(album_id).await
    }

    async fn save(&self, album_stats: AlbumStats) -> Result<(), ModelError> {
        let id_i64 = album_stats.album_id.as_i64();
        let wrapper = Arc::new(AlbumStatsWrapper(album_stats));

        self.memtable_context
            .insert(id_i64, wrapper)
            .await
            .map_err(|e| ModelError::DbErr(format!("Failed to insert into memtable: {}", e)))?;

        Ok(())
    }

    async fn delete_by_album_id(&self, album_id: AlbumId) -> Result<(), ModelError> {
        let id_i64 = album_id.as_i64();

        // 从 memtable 中添加 tombstone
        self.memtable_context
            .delete(&id_i64)
            .await
            .map_err(|e| ModelError::DbErr(format!("Failed to delete: {}", e)))?;

        // 从数据库删除
        self.inner.delete_by_album_id(album_id).await
    }
}
