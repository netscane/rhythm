use async_trait::async_trait;
use domain::value::{ArtistId, ParticipantRole};
use log::info;
use model::participant_stats::{ParticipantStats, ParticipantStatsRepository};
use model::ModelError;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::super::memtable::{
    IndexValue, IndexMatch, Memtable, MemtableContext, MemtablePersister, MemtableValue,
};

// 使用复合键：artist_id + role 的字符串表示
fn make_composite_key(artist_id: &ArtistId, role: &ParticipantRole) -> String {
    format!("{}:{:?}", artist_id.as_i64(), role)
}

#[derive(Clone)]
struct ParticipantStatsWrapper(ParticipantStats);

impl MemtableValue<String> for ParticipantStatsWrapper {
    fn get_key(&self) -> String {
        make_composite_key(&self.0.artist_id, &self.0.role)
    }

    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)> {
        vec![]
    }

    fn get_index(&self, _index_name: &str) -> IndexValue {
        panic!("No indexes defined for ParticipantStats")
    }
}

pub struct ParticipantStatsPersister<R>
where
    R: ParticipantStatsRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
}

impl<R> Clone for ParticipantStatsPersister<R>
where
    R: ParticipantStatsRepository + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<R> ParticipantStatsPersister<R>
where
    R: ParticipantStatsRepository + Send + Sync + 'static,
{
    pub fn new(inner: Arc<R>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<R> MemtablePersister<String, ParticipantStatsWrapper> for ParticipantStatsPersister<R>
where
    R: ParticipantStatsRepository + Send + Sync + 'static,
{
    async fn persist(&self, _key: String, value: Arc<ParticipantStatsWrapper>) -> Result<(), String> {
        let participant_stats = &value.0;

        // Use the unified adjust method to update all fields in a single operation
        self.inner
            .adjust_participant_stats(participant_stats.clone())
            .await
            .map_err(|e| format!("Failed to adjust participant stats: {}", e))?;

        Ok(())
    }

    async fn remove(&self, key: String) -> Result<(), String> {
        // 解析复合键
        let parts: Vec<&str> = key.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid composite key: {}", key));
        }

        let artist_id_i64: i64 = parts[0]
            .parse()
            .map_err(|e| format!("Failed to parse artist_id: {}", e))?;
        let artist_id = ArtistId::from(artist_id_i64);

        // 简单解析 role（注意：这里假设 role 的 Debug 输出格式）
        let role = match parts[1] {
            "AlbumArtist" => ParticipantRole::AlbumArtist,
            "Artist" => ParticipantRole::Artist,
            "Performer" => ParticipantRole::Performer,
            _ => return Err(format!("Unknown role: {}", parts[1])),
        };

        // 创建删除条目（所有字段都为负数）
        let delete_entry = ParticipantStats {
            artist_id,
            role,
            duration: -1, // 删除所有时长
            size: -1, // 删除所有大小
            song_count: -1, // 删除所有歌曲
            album_count: -1, // 删除所有专辑
        };

        self.inner
            .adjust_participant_stats(delete_entry)
            .await
            .map_err(|e| format!("Failed to adjust participant stats for removal: {}", e))?;

        Ok(())
    }
}

pub struct BufferedParticipantStatsRepository<R>
where
    R: ParticipantStatsRepository + Send + Sync + 'static,
{
    memtable_context: Arc<MemtableContext<String, ParticipantStatsWrapper, ParticipantStatsPersister<R>>>,
    inner: Arc<R>,
}

impl<R> BufferedParticipantStatsRepository<R>
where
    R: ParticipantStatsRepository + Send + Sync + 'static,
{
    pub fn new(inner: R, cache_capacity: usize, flush_timeout: Duration) -> Arc<Self> {
        let memtable_size_threshold = cache_capacity.max(100);

        // 创建 persister
        let inner_arc = Arc::new(inner);
        let persister = Arc::new(ParticipantStatsPersister::new(inner_arc.clone()));

        // 创建 memtable 和相关组件
        let active_memtable = Arc::new(RwLock::new(Memtable::<String, ParticipantStatsWrapper>::new()));
        let active_size = Arc::new(AtomicUsize::new(0));

        let memtable_context = Arc::new(MemtableContext::new(
            "ParticipantStats".to_string(),
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
        info!("Starting graceful shutdown of BufferedParticipantStatsRepository");

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
impl<R> ParticipantStatsRepository for BufferedParticipantStatsRepository<R>
where
    R: ParticipantStatsRepository + Send + Sync + 'static,
{
    async fn adjust_participant_stats(&self, entry: ParticipantStats) -> Result<(), ModelError> {
        let key = make_composite_key(&entry.artist_id, &entry.role);

        // 使用 update_or_insert 保证原子性，避免竞态条件
        self.memtable_context
            .update_or_insert(key, |current| {
                let (new_duration, new_size, new_song_count, new_album_count) = match current {
                    Some(existing) => (
                        existing.0.duration + entry.duration,
                        existing.0.size + entry.size,
                        existing.0.song_count + entry.song_count,
                        existing.0.album_count + entry.album_count,
                    ),
                    None => (entry.duration, entry.size, entry.song_count, entry.album_count),
                };

                Arc::new(ParticipantStatsWrapper(ParticipantStats {
                    artist_id: entry.artist_id.clone(),
                    role: entry.role.clone(),
                    duration: new_duration,
                    size: new_size,
                    song_count: new_song_count,
                    album_count: new_album_count,
                }))
            })
            .await
            .map_err(|e| ModelError::DbErr(format!("Failed to update memtable: {}", e)))?;

        Ok(())
    }
}
