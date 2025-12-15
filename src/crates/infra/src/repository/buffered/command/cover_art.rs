use async_trait::async_trait;
use domain::cover_art::{CoverArt, CoverArtError, CoverArtRepository};
use domain::value::{AlbumId, CoverArtId};
use futures::stream::{self, StreamExt};
use log::{error, info, warn};
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};

// 导入通用 memtable 模块
use super::super::memtable::{
    IndexValue, IndexMatch, Memtable, MemtableContext, MemtablePersister, MemtableValue,
};

#[derive(Clone)]
struct CoverArtWrapper(CoverArt);

// 为 CoverArt 实现 MemtableValue
impl MemtableValue<i64> for CoverArtWrapper {
    fn get_key(&self) -> i64 {
        self.0.id.as_i64()
    }

    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)> {
        // CoverArt 主要通过 album_id 查询，但这需要额外的处理
        // 暂时不添加索引，因为 find_by_album_id 需要特殊处理
        vec![]
    }

    fn get_index(&self, _index_name: &str) -> IndexValue {
        // CoverArt 暂时没有索引
        panic!("CoverArt does not support indexes in memtable");
    }
}

pub struct LruCacheCoverArtRepository<R>
where
    R: CoverArtRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
    lru_cache: Arc<RwLock<LruCache<i64, CoverArt>>>,
}

impl<R> LruCacheCoverArtRepository<R>
where
    R: CoverArtRepository + Send + Sync + 'static,
{
    pub fn new(inner: Arc<R>, cache_capacity: usize) -> Arc<Self> {
        let lru_capacity = NonZeroUsize::new(cache_capacity).unwrap();
        let lru_cache = Arc::new(RwLock::new(LruCache::new(lru_capacity)));
        Arc::new(Self { inner, lru_cache })
    }
}

#[async_trait]
impl<R> CoverArtRepository for LruCacheCoverArtRepository<R>
where
    R: CoverArtRepository + Send + Sync + 'static,
{
    async fn save(&self, cover_art: CoverArt) -> Result<CoverArt, CoverArtError> {
        let result = self.inner.save(cover_art).await?;

        // 缓存结果
        let id_i64 = result.id.as_i64();
        {
            let mut lru_cache = self.lru_cache.write().await;
            lru_cache.put(id_i64, result.clone());
        }
        Ok(result)
    }

    async fn find_by_id(&self, id: &CoverArtId) -> Result<Option<CoverArt>, CoverArtError> {
        let id_i64 = id.as_i64();

        // 先查 LRU cache
        {
            let mut lru_cache = self.lru_cache.write().await;
            if let Some(cover_art) = lru_cache.get(&id_i64) {
                return Ok(Some(cover_art.clone()));
            }
        }

        // 查数据库
        match self.inner.find_by_id(id).await {
            Ok(Some(cover_art)) => {
                let id_i64 = cover_art.id.as_i64();
                {
                    let mut lru_cache = self.lru_cache.write().await;
                    lru_cache.put(id_i64, cover_art.clone());
                }
                Ok(Some(cover_art))
            }
            result => result,
        }
    }

    async fn find_by_album_id(&self, album_id: &AlbumId) -> Result<Vec<CoverArt>, CoverArtError> {
        // find_by_album_id 直接查询数据库，因为需要聚合多个结果
        self.inner.find_by_album_id(album_id).await
    }

    async fn delete(&self, id: &CoverArtId) -> Result<(), CoverArtError> {
        let id_i64 = id.as_i64();

        // 从 LRU cache 中移除
        {
            let mut lru_cache = self.lru_cache.write().await;
            lru_cache.pop(&id_i64);
        }

        // 从数据库删除
        self.inner.delete(id).await
    }
}

// CoverArt Persister：实现 MemtablePersister trait
pub struct CoverArtPersister<R>
where
    R: CoverArtRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
    save_semaphore: Arc<Semaphore>,
    concurrency: usize,
}

impl<R> Clone for CoverArtPersister<R>
where
    R: CoverArtRepository + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            save_semaphore: self.save_semaphore.clone(),
            concurrency: self.concurrency,
        }
    }
}

impl<R> CoverArtPersister<R>
where
    R: CoverArtRepository + Send + Sync + 'static,
{
    pub fn new(inner: Arc<R>, concurrency: usize) -> Self {
        Self {
            inner,
            save_semaphore: Arc::new(Semaphore::new(concurrency)),
            concurrency,
        }
    }
}

#[async_trait]
impl<R> MemtablePersister<i64, CoverArtWrapper> for CoverArtPersister<R>
where
    R: CoverArtRepository + Send + Sync + 'static,
{
    async fn persist(&self, key: i64, value: Arc<CoverArtWrapper>) -> Result<(), String> {
        let _permit = self
            .save_semaphore
            .acquire()
            .await
            .map_err(|e| format!("Failed to acquire semaphore: {}", e))?;

        let mut cover_art = value.0.clone();
        cover_art.version -= 1;

        // 重试机制
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            match self.inner.save(cover_art.clone()).await {
                Ok(_) => {
                    //info!("Successfully saved cover art {} to database", key);
                    return Ok(());
                }
                Err(e) => {
                    let error_msg = format!("{}", e);

                    // 版本冲突不是错误：表示数据已经被成功保存过了
                    if error_msg.contains("Version conflict") {
                        info!(
                            "CoverArt {} already saved (version conflict), skipping",
                            key
                        );
                        return Ok(());
                    }

                    // 连接池超时错误：进行重试
                    if error_msg.contains("Connection pool timed out")
                        || error_msg.contains("pool timed out")
                    {
                        if retry_count < MAX_RETRIES {
                            let delay_ms = 20 * (1 << retry_count);
                            warn!(
                                "Connection pool timeout for cover art {}, retrying in {}ms (attempt {}/{})",
                                key, delay_ms, retry_count + 1, MAX_RETRIES
                            );
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            retry_count += 1;
                            continue;
                        }
                    }

                    // 其他错误：记录并返回错误
                    error!("Failed to save cover art {} to database: {}", key, e);
                    return Err(format!("Failed to save: {}", e));
                }
            }
        }
    }

    async fn remove(&self, key: i64) -> Result<(), String> {
        let _permit = self
            .save_semaphore
            .acquire()
            .await
            .map_err(|e| format!("Failed to acquire semaphore: {}", e))?;

        let cover_art_id = CoverArtId::from(key);

        self.inner
            .delete(&cover_art_id)
            .await
            .map_err(|e| format!("Failed to delete: {}", e))?;

        info!("Successfully deleted cover art {} from database", key);
        Ok(())
    }

    async fn persist_batch(&self, items: Vec<(i64, Arc<CoverArtWrapper>)>) -> Result<(), String> {
        let start_time = Instant::now();
        info!("Starting batch persist for {} cover arts", items.len());

        stream::iter(items)
            .map(|(key, value)| {
                let persister = self.clone();
                async move {
                    if let Err(e) = persister.persist(key, value).await {
                        error!("Failed to persist cover art {}: {}", key, e);
                    }
                }
            })
            .buffer_unordered(self.concurrency)
            .collect::<Vec<_>>()
            .await;

        let end_time = Instant::now();
        let duration = end_time.duration_since(start_time);
        info!("Batch persist completed in {}ms", duration.as_millis());
        Ok(())
    }

    async fn remove_batch(&self, keys: Vec<i64>) -> Result<(), String> {
        let mut handles = vec![];

        for key in keys {
            let persister = self.clone();
            let handle = tokio::spawn(async move { persister.remove(key).await });
            handles.push(handle);
        }

        for handle in handles {
            if let Err(e) = handle.await {
                error!("Task failed: {}", e);
            }
        }

        Ok(())
    }
}

pub struct BufferedCoverArtRepository<R>
where
    R: CoverArtRepository + Send + Sync + 'static,
{
    memtable_context: Arc<
        MemtableContext<i64, CoverArtWrapper, CoverArtPersister<LruCacheCoverArtRepository<R>>>,
    >,
    inner: Arc<LruCacheCoverArtRepository<R>>,
}

impl<R> BufferedCoverArtRepository<R>
where
    R: CoverArtRepository + Send + Sync + 'static,
{
    pub fn new(
        inner: R,
        cache_capacity: usize,
        concurrency: usize,
        flush_timeout: Duration,
    ) -> Arc<Self> {
        let memtable_size_threshold = cache_capacity.max(100);

        // 创建 persister
        let inner_arc = LruCacheCoverArtRepository::new(Arc::new(inner), cache_capacity);
        let persister = Arc::new(CoverArtPersister::new(inner_arc.clone(), concurrency));

        let active_memtable = Arc::new(RwLock::new(Memtable::<i64, CoverArtWrapper>::new()));
        let active_size = Arc::new(AtomicUsize::new(0));

        let memtable_context = Arc::new(MemtableContext::new(
            "CoverArt".to_string(),
            active_memtable,
            active_size,
            memtable_size_threshold,
            persister,
            flush_timeout,
        ));

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
        info!("Starting graceful shutdown of BufferedCoverArtRepository");

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
impl<R> CoverArtRepository for BufferedCoverArtRepository<R>
where
    R: CoverArtRepository + Send + Sync + 'static,
{
    async fn save(&self, mut cover_art: CoverArt) -> Result<CoverArt, CoverArtError> {
        cover_art.version += 1;
        let id_i64 = cover_art.id.as_i64();
        let cover_art_wrapper = Arc::new(CoverArtWrapper(cover_art.clone()));

        self.memtable_context
            .insert(id_i64, cover_art_wrapper)
            .await
            .map_err(|e| CoverArtError::DatabaseError {
                message: format!("Failed to insert into memtable: {}", e),
            })?;

        Ok(cover_art)
    }

    async fn find_by_id(&self, id: &CoverArtId) -> Result<Option<CoverArt>, CoverArtError> {
        let id_i64 = id.as_i64();

        // 先查 active memtable
        if let Some(cover_art_wrapper) = self.memtable_context.get(&id_i64).await {
            return Ok(Some(cover_art_wrapper.0.clone()));
        }

        // 查 LRU cache 和数据库
        self.inner.find_by_id(id).await
    }

    async fn find_by_album_id(&self, album_id: &AlbumId) -> Result<Vec<CoverArt>, CoverArtError> {
        // find_by_album_id 需要聚合多个结果，直接查询数据库
        // TODO: 未来可以优化，在 memtable 中也查询
        self.inner.find_by_album_id(album_id).await
    }

    async fn delete(&self, id: &CoverArtId) -> Result<(), CoverArtError> {
        let id_i64 = id.as_i64();

        self.memtable_context
            .delete(&id_i64)
            .await
            .map_err(|e| CoverArtError::DatabaseError {
                message: format!("Failed to delete cover art {}: {}", id_i64, e),
            })?;

        self.inner.delete(id).await
    }
}
