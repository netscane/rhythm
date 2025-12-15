use async_trait::async_trait;
use domain::value::GenreId;
use log::info;
use model::genre::{GenreStats, GenreStatsRepository};
use model::ModelError;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::super::memtable::{
    IndexMatch, IndexValue, Memtable, MemtableContext, MemtablePersister, MemtableValue,
};

#[derive(Clone)]
struct GenreStatsWrapper(GenreStats);

impl MemtableValue<i64> for GenreStatsWrapper {
    fn get_key(&self) -> i64 {
        self.0.genre_id.as_i64()
    }

    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)> {
        vec![]
    }

    fn get_index(&self, _index_name: &str) -> IndexValue {
        panic!("No indexes defined for GenreStats")
    }
}

pub struct GenreStatsPersister<R>
where
    R: GenreStatsRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
}

impl<R> Clone for GenreStatsPersister<R>
where
    R: GenreStatsRepository + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<R> GenreStatsPersister<R>
where
    R: GenreStatsRepository + Send + Sync + 'static,
{
    pub fn new(inner: Arc<R>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<R> MemtablePersister<i64, GenreStatsWrapper> for GenreStatsPersister<R>
where
    R: GenreStatsRepository + Send + Sync + 'static,
{
    async fn persist(&self, _key: i64, value: Arc<GenreStatsWrapper>) -> Result<(), String> {
        let genre_stats = &value.0;

        // Use unified adjust_stats method
        self.inner
            .adjust_stats(genre_stats.clone())
            .await
            .map_err(|e| format!("Failed to adjust stats: {}", e))?;

        Ok(())
    }

    async fn remove(&self, key: i64) -> Result<(), String> {
        let genre_id = GenreId::from(key);

        // Create delete entry with negative counts
        let delete_entry = GenreStats {
            genre_id,
            song_count: -1,
            album_count: -1,
        };

        self.inner
            .adjust_stats(delete_entry)
            .await
            .map_err(|e| format!("Failed to adjust stats for removal: {}", e))?;

        Ok(())
    }
}

pub struct BufferedGenreStatsRepository<R>
where
    R: GenreStatsRepository + Send + Sync + 'static,
{
    memtable_context: Arc<MemtableContext<i64, GenreStatsWrapper, GenreStatsPersister<R>>>,
    inner: Arc<R>,
}

impl<R> BufferedGenreStatsRepository<R>
where
    R: GenreStatsRepository + Send + Sync + 'static,
{
    pub fn new(inner: R, cache_capacity: usize, flush_timeout: Duration) -> Arc<Self> {
        let memtable_size_threshold = cache_capacity.max(100);

        // 创建 persister
        let inner_arc = Arc::new(inner);
        let persister = Arc::new(GenreStatsPersister::new(inner_arc.clone()));

        // 创建 memtable 和相关组件
        let active_memtable = Arc::new(RwLock::new(Memtable::<i64, GenreStatsWrapper>::new()));
        let active_size = Arc::new(AtomicUsize::new(0));

        let memtable_context = Arc::new(MemtableContext::new(
            "GenreStats".to_string(),
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
        info!("Starting graceful shutdown of BufferedGenreStatsRepository");

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
impl<R> GenreStatsRepository for BufferedGenreStatsRepository<R>
where
    R: GenreStatsRepository + Send + Sync + 'static,
{
    async fn adjust_stats(&self, entry: GenreStats) -> Result<(), ModelError> {
        let key = entry.genre_id.as_i64();

        // 使用 update_or_insert 保证原子性，避免竞态条件
        self.memtable_context
            .update_or_insert(key, |current| {
                let (new_song_count, new_album_count) = match current {
                    Some(existing) => (
                        existing.0.song_count + entry.song_count,
                        existing.0.album_count + entry.album_count,
                    ),
                    None => (entry.song_count, entry.album_count),
                };

                Arc::new(GenreStatsWrapper(GenreStats {
                    genre_id: entry.genre_id.clone(),
                    song_count: new_song_count,
                    album_count: new_album_count,
                }))
            })
            .await
            .map_err(|e| ModelError::DbErr(format!("Failed to update memtable: {}", e)))?;

        Ok(())
    }
}
