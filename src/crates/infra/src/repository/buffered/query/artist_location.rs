use async_trait::async_trait;
use domain::value::{ArtistId, MediaPath};
use log::info;
use model::artist_location::{ArtistLocation, ArtistLocationRepository};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::super::memtable::{
    IndexMatch, IndexValue, Memtable, MemtableContext, MemtablePersister, MemtableValue,
};

// 复合主键: (artist_id, location_protocol, location_path)
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct LocationKey {
    artist_id: i64,
    protocol: String,
    path: String,
}

impl super::super::memtable::MemtableKey for LocationKey {
    fn min_value() -> Self {
        Self {
            artist_id: i64::MIN,
            protocol: String::new(),
            path: String::new(),
        }
    }
}

#[derive(Clone)]
struct ArtistLocationWrapper(ArtistLocation);

impl MemtableValue<LocationKey> for ArtistLocationWrapper {
    fn get_key(&self) -> LocationKey {
        LocationKey {
            artist_id: i64::from(self.0.artist_id.clone()),
            protocol: self.0.location.protocol.clone(),
            path: self.0.location.path.clone(),
        }
    }

    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)> {
        // 不需要额外索引,因为不需要读取操作
        vec![]
    }

    fn get_index(&self, _index_name: &str) -> IndexValue {
        panic!("No indexes defined for ArtistLocation")
    }
}

pub struct ArtistLocationPersister<R>
where
    R: ArtistLocationRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
}

impl<R> Clone for ArtistLocationPersister<R>
where
    R: ArtistLocationRepository + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<R> ArtistLocationPersister<R>
where
    R: ArtistLocationRepository + Send + Sync + 'static,
{
    pub fn new(inner: Arc<R>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<R> MemtablePersister<LocationKey, ArtistLocationWrapper> for ArtistLocationPersister<R>
where
    R: ArtistLocationRepository + Send + Sync + 'static,
{
    async fn persist(
        &self,
        _key: LocationKey,
        value: Arc<ArtistLocationWrapper>,
    ) -> Result<(), String> {
        self.inner
            .adjust_count(value.0.clone())
            .await
            .map_err(|e| format!("Failed to adjust count: {}", e))?;
        Ok(())
    }

    async fn remove(&self, key: LocationKey) -> Result<(), String> {
        let entry = ArtistLocation {
            artist_id: ArtistId::from(key.artist_id),
            location: MediaPath {
                protocol: key.protocol,
                path: key.path,
            },
            total: -1, // Decrement by 1
            update_time: None, // Will be set by repository
        };
        self.inner
            .adjust_count(entry)
            .await
            .map_err(|e| format!("Failed to adjust count: {}", e))?;
        Ok(())
    }
}

pub struct BufferedArtistLocationRepository<R>
where
    R: ArtistLocationRepository + Send + Sync + 'static,
{
    memtable_context:
        Arc<MemtableContext<LocationKey, ArtistLocationWrapper, ArtistLocationPersister<R>>>,
    inner: Arc<R>,
}

impl<R> BufferedArtistLocationRepository<R>
where
    R: ArtistLocationRepository + Send + Sync + 'static,
{
    pub fn new(inner: R, cache_capacity: usize, flush_timeout: Duration) -> Arc<Self> {
        let memtable_size_threshold = cache_capacity.max(100);

        // 创建 persister
        let inner_arc = Arc::new(inner);
        let persister = Arc::new(ArtistLocationPersister::new(inner_arc.clone()));

        // 创建 memtable 和相关组件
        let active_memtable = Arc::new(RwLock::new(
            Memtable::<LocationKey, ArtistLocationWrapper>::new(),
        ));
        let active_size = Arc::new(AtomicUsize::new(0));

        let memtable_context = Arc::new(MemtableContext::new(
            "ArtistLocation".to_string(),
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
        info!("Starting graceful shutdown of BufferedArtistLocationRepository");

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
impl<R> ArtistLocationRepository for BufferedArtistLocationRepository<R>
where
    R: ArtistLocationRepository + Send + Sync + 'static,
{
    async fn adjust_count(
        &self,
        entry: ArtistLocation,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = LocationKey {
            artist_id: i64::from(entry.artist_id.clone()),
            protocol: entry.location.protocol.clone(),
            path: entry.location.path.clone(),
        };

        // 使用 update_or_insert 保证原子性，避免竞态条件
        self.memtable_context
            .update_or_insert(key, |current| {
                let new_count = match current {
                    Some(existing) => existing.0.total + entry.total,
                    None => entry.total,
                };

                Arc::new(ArtistLocationWrapper(ArtistLocation {
                    artist_id: entry.artist_id.clone(),
                    location: entry.location.clone(),
                    total: new_count,
                    update_time: entry.update_time.clone(),
                }))
            })
            .await
            .map_err(|e| {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                    as Box<dyn std::error::Error + Send + Sync>
            })?;

        Ok(())
    }
}
