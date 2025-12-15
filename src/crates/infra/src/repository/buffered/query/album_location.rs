use async_trait::async_trait;
use domain::value::{AlbumId, MediaPath};
use log::info;
use model::album_location::{AlbumLocation, AlbumLocationRepository};
use model::ModelError;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::super::memtable::{
    IndexMatch, IndexValue, Memtable, MemtableContext, MemtablePersister, MemtableValue,
};

// 复合主键: (album_id, location_protocol, location_path)
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct LocationKey {
    album_id: i64,
    protocol: String,
    path: String,
}

impl super::super::memtable::MemtableKey for LocationKey {
    fn min_value() -> Self {
        Self {
            album_id: i64::MIN,
            protocol: String::new(),
            path: String::new(),
        }
    }
}

#[derive(Clone)]
struct AlbumLocationWrapper(AlbumLocation);

impl MemtableValue<LocationKey> for AlbumLocationWrapper {
    fn get_key(&self) -> LocationKey {
        LocationKey {
            album_id: i64::from(self.0.album_id.clone()),
            protocol: self.0.location.protocol.clone(),
            path: self.0.location.path.clone(),
        }
    }

    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)> {
        // 不需要额外索引,因为不需要读取操作
        vec![]
    }

    fn get_index(&self, _index_name: &str) -> IndexValue {
        panic!("No indexes defined for AlbumLocation")
    }
}

pub struct AlbumLocationPersister<R>
where
    R: AlbumLocationRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
}

impl<R> Clone for AlbumLocationPersister<R>
where
    R: AlbumLocationRepository + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<R> AlbumLocationPersister<R>
where
    R: AlbumLocationRepository + Send + Sync + 'static,
{
    pub fn new(inner: Arc<R>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<R> MemtablePersister<LocationKey, AlbumLocationWrapper> for AlbumLocationPersister<R>
where
    R: AlbumLocationRepository + Send + Sync + 'static,
{
    async fn persist(
        &self,
        _key: LocationKey,
        value: Arc<AlbumLocationWrapper>,
    ) -> Result<(), String> {
        self.inner
            .adjust_count(value.0.clone())
            .await
            .map_err(|e| format!("Failed to adjust count: {}", e))?;
        Ok(())
    }

    async fn remove(&self, key: LocationKey) -> Result<(), String> {
        let entry = AlbumLocation {
            album_id: AlbumId::from(key.album_id),
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

pub struct BufferedAlbumLocationRepository<R>
where
    R: AlbumLocationRepository + Send + Sync + 'static,
{
    memtable_context:
        Arc<MemtableContext<LocationKey, AlbumLocationWrapper, AlbumLocationPersister<R>>>,
    inner: Arc<R>,
}

impl<R> BufferedAlbumLocationRepository<R>
where
    R: AlbumLocationRepository + Send + Sync + 'static,
{
    pub fn new(inner: R, cache_capacity: usize, flush_timeout: Duration) -> Arc<Self> {
        let memtable_size_threshold = cache_capacity.max(100);

        // 创建 persister
        let inner_arc = Arc::new(inner);
        let persister = Arc::new(AlbumLocationPersister::new(inner_arc.clone()));

        // 创建 memtable 和相关组件
        let active_memtable = Arc::new(RwLock::new(
            Memtable::<LocationKey, AlbumLocationWrapper>::new(),
        ));
        let active_size = Arc::new(AtomicUsize::new(0));

        let memtable_context = Arc::new(MemtableContext::new(
            "AlbumLocation".to_string(),
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
        info!("Starting graceful shutdown of BufferedAlbumLocationRepository");

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
impl<R> AlbumLocationRepository for BufferedAlbumLocationRepository<R>
where
    R: AlbumLocationRepository + Send + Sync + 'static,
{
    async fn adjust_count(&self, entry: AlbumLocation) -> Result<(), ModelError> {
        let key = LocationKey {
            album_id: i64::from(entry.album_id.clone()),
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

                Arc::new(AlbumLocationWrapper(AlbumLocation {
                    album_id: entry.album_id.clone(),
                    location: entry.location.clone(),
                    total: new_count,
                    update_time: entry.update_time.clone(),
                }))
            })
            .await
            .map_err(|e| ModelError::DbErr(format!("Failed to update memtable: {}", e)))?;

        Ok(())
    }
}
