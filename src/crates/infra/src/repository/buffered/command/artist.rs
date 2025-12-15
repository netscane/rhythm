use async_trait::async_trait;
use domain::artist::{Artist, ArtistError, ArtistRepository};
use domain::value::ArtistId;
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
struct ArtistWrapper(Artist);

// 为 Artist 实现 MemtableValue
impl MemtableValue<i64> for ArtistWrapper {
    fn get_key(&self) -> i64 {
        self.0.id.as_i64()
    }

    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)> {
        vec![
            ("sort_name", IndexValue::String(self.0.sort_name.clone()), IndexMatch::Exact),
        ]
    }

    fn get_index(&self, index_name: &str) -> IndexValue {
        match index_name {
            "sort_name" => IndexValue::String(self.0.sort_name.clone()),
            _ => panic!("Invalid index name: {}", index_name),
        }
    }
}

pub struct LruCacheArtistRepository<R>
where
    R: ArtistRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
    lru_cache: Arc<RwLock<LruCache<i64, Artist>>>,
    lru_sort_name_index: Arc<RwLock<HashMap<String, i64>>>,
}

impl<R> LruCacheArtistRepository<R>
where
    R: ArtistRepository + Send + Sync + 'static,
{
    pub fn new(inner: Arc<R>, cache_capacity: usize) -> Arc<Self> {
        let lru_capacity = NonZeroUsize::new(cache_capacity).unwrap();
        let lru_cache = Arc::new(RwLock::new(LruCache::new(lru_capacity)));
        let lru_sort_name_index = Arc::new(RwLock::new(HashMap::new()));
        Arc::new(Self {
            inner,
            lru_cache,
            lru_sort_name_index,
        })
    }
}

#[async_trait]
impl<R> ArtistRepository for LruCacheArtistRepository<R>
where
    R: ArtistRepository + Send + Sync + 'static,
{
    async fn save(&self, artist: Artist) -> Result<Artist, ArtistError> {
        let result = self.inner.save(artist).await?;

        // 缓存结果
        let id_i64 = result.id.as_i64();
        let sort_name = result.sort_name.clone();
        
        // ⚠️ 修复死锁：分开获取两个写锁
        {
            let mut lru_cache = self.lru_cache.write().await;
            lru_cache.put(id_i64, result.clone());
        }
        {
            let mut lru_sort_name_index = self.lru_sort_name_index.write().await;
            lru_sort_name_index.insert(sort_name, id_i64);
        }
        
        Ok(result)
    }

    async fn by_id(&self, id: ArtistId) -> Result<Option<Artist>, ArtistError> {
        let id_i64 = id.as_i64();

        // 先查 LRU cache
        {
            let mut lru_cache = self.lru_cache.write().await;
            if let Some(artist) = lru_cache.get(&id_i64) {
                return Ok(Some(artist.clone()));
            }
        }

        // 查数据库
        match self.inner.by_id(id).await {
            Ok(Some(artist)) => {
                let id_i64 = artist.id.as_i64();
                let sort_name = artist.sort_name.clone();
                
                // ⚠️ 修复死锁：分开获取两个写锁
                {
                    let mut lru_cache = self.lru_cache.write().await;
                    lru_cache.put(id_i64, artist.clone());
                }
                {
                    let mut lru_sort_name_index = self.lru_sort_name_index.write().await;
                    lru_sort_name_index.insert(sort_name, id_i64);
                }
                
                Ok(Some(artist))
            }
            result => result,
        }
    }

    async fn find_by_sort_name(&self, sort_name: &String) -> Result<Option<Artist>, ArtistError> {
        // 先查 LRU cache
        {
            let lru_sort_name_index = self.lru_sort_name_index.read().await;
            if let Some(id) = lru_sort_name_index.get(sort_name) {
                let mut lru_cache = self.lru_cache.write().await;
                if let Some(artist) = lru_cache.get(id) {
                    return Ok(Some(artist.clone()));
                }
            }
        }

        // 查数据库并缓存结果
        match self.inner.find_by_sort_name(sort_name).await {
            Ok(Some(artist)) => {
                let id_i64 = artist.id.as_i64();
                let sort_name = artist.sort_name.clone();
                
                // ⚠️ 修复死锁：分开获取两个写锁
                {
                    let mut lru_cache = self.lru_cache.write().await;
                    lru_cache.put(id_i64, artist.clone());
                }
                {
                    let mut lru_sort_name_index = self.lru_sort_name_index.write().await;
                    lru_sort_name_index.insert(sort_name, id_i64);
                }
                
                Ok(Some(artist))
            }
            result => result,
        }
    }

    async fn delete(&self, artist_id: ArtistId) -> Result<(), ArtistError> {
        let id_i64 = artist_id.as_i64();

        // 从 LRU cache 中移除
        // ⚠️ 修复死锁：先获取值，然后分开获取两个写锁
        let sort_name_opt = {
            let mut lru_cache = self.lru_cache.write().await;
            lru_cache.pop(&id_i64).map(|artist| artist.sort_name)
        };
        
        if let Some(sort_name) = sort_name_opt {
            let mut lru_sort_name_index = self.lru_sort_name_index.write().await;
            lru_sort_name_index.remove(&sort_name);
        }

        // 从数据库删除
        self.inner.delete(artist_id).await
    }
}

// Artist Persister：实现 MemtablePersister trait
pub struct ArtistPersister<R>
where
    R: ArtistRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
    save_semaphore: Arc<Semaphore>,
    concurrency: usize,
}

impl<R> Clone for ArtistPersister<R>
where
    R: ArtistRepository + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            save_semaphore: self.save_semaphore.clone(),
            concurrency: self.concurrency,
        }
    }
}

impl<R> ArtistPersister<R>
where
    R: ArtistRepository + Send + Sync + 'static,
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
impl<R> MemtablePersister<i64, ArtistWrapper> for ArtistPersister<R>
where
    R: ArtistRepository + Send + Sync + 'static,
{
    async fn persist(&self, key: i64, value: Arc<ArtistWrapper>) -> Result<(), String> {
        let _permit = self
            .save_semaphore
            .acquire()
            .await
            .map_err(|e| format!("Failed to acquire semaphore: {}", e))?;

        let mut artist = value.0.clone();
        artist.version -= 1;

        // 重试机制
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            match self.inner.save(artist.clone()).await {
                Ok(_) => {
                    //info!("Successfully saved artist {} to database", key);
                    return Ok(());
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    
                    // 版本冲突不是错误：表示数据已经被成功保存过了
                    if error_msg.contains("Version conflict") {
                        info!("Artist {} already saved (version conflict), skipping", key);
                        return Ok(());
                    }
                    
                    // 连接池超时错误：进行重试
                    if error_msg.contains("Connection pool timed out")
                        || error_msg.contains("pool timed out")
                    {
                        if retry_count < MAX_RETRIES {
                            let delay_ms = 20 * (1 << retry_count);
                            warn!(
                                "Connection pool timeout for artist {}, retrying in {}ms (attempt {}/{})",
                                key, delay_ms, retry_count + 1, MAX_RETRIES
                            );
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            retry_count += 1;
                            continue;
                        }
                    }
                    
                    // 其他错误：记录并返回错误
                    error!("Failed to save artist {} to database: {}", key, e);
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

        let artist_id = ArtistId::from(key);

        self.inner
            .delete(artist_id)
            .await
            .map_err(|e| format!("Failed to delete: {}", e))?;

        info!("Successfully deleted artist {} from database", key);
        Ok(())
    }

    async fn persist_batch(&self, items: Vec<(i64, Arc<ArtistWrapper>)>) -> Result<(), String> {
        let start_time = Instant::now();
        info!("Starting batch persist for {} artists", items.len());

        stream::iter(items)
            .map(|(key, value)| {
                let persister = self.clone();
                async move {
                    if let Err(e) = persister.persist(key, value).await {
                        error!("Failed to persist artist {}: {}", key, e);
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

pub struct BufferedArtistRepository<R>
where
    R: ArtistRepository + Send + Sync + 'static,
{
    memtable_context:
        Arc<MemtableContext<i64, ArtistWrapper, ArtistPersister<LruCacheArtistRepository<R>>>>,
    inner: Arc<LruCacheArtistRepository<R>>,
}

impl<R> BufferedArtistRepository<R>
where
    R: ArtistRepository + Send + Sync + 'static,
{
    pub fn new(
        inner: R,
        cache_capacity: usize,
        concurrency: usize,
        flush_timeout: Duration,
    ) -> Arc<Self> {
        let memtable_size_threshold = cache_capacity.max(100);

        // 创建 persister
        let inner_arc = LruCacheArtistRepository::new(Arc::new(inner), cache_capacity);
        let persister = Arc::new(ArtistPersister::new(inner_arc.clone(), concurrency));

        let active_memtable = Arc::new(RwLock::new(Memtable::<i64, ArtistWrapper>::new()));
        let active_size = Arc::new(AtomicUsize::new(0));

        let memtable_context = Arc::new(MemtableContext::new(
            "Artist".to_string(),
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
        info!("Starting graceful shutdown of BufferedArtistRepository");

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
impl<R> ArtistRepository for BufferedArtistRepository<R>
where
    R: ArtistRepository + Send + Sync + 'static,
{
    async fn save(&self, mut artist: Artist) -> Result<Artist, ArtistError> {
        artist.version += 1;
        let id_i64 = artist.id.as_i64();
        let artist_wrapper = Arc::new(ArtistWrapper(artist.clone()));

        self.memtable_context
            .insert(id_i64, artist_wrapper)
            .await
            .map_err(|e| ArtistError::DbErr(format!("Failed to insert into memtable: {}", e)))?;

        Ok(artist)
    }

    async fn by_id(&self, id: ArtistId) -> Result<Option<Artist>, ArtistError> {
        let id_i64 = id.as_i64();

        // 先查 active memtable
        if let Some(artist_wrapper) = self.memtable_context.get(&id_i64).await {
            return Ok(Some(artist_wrapper.0.clone()));
        }

        // 查 LRU cache 和数据库
        self.inner.by_id(id).await
    }

    async fn find_by_sort_name(&self, sort_name: &String) -> Result<Option<Artist>, ArtistError> {
        // 先查 active memtable 按 sort_name 查询
        if let Some(artist_wrapper) = self
            .memtable_context
            .get_by_index("sort_name", IndexValue::String(sort_name.clone()))
            .await
        {
            return Ok(Some(artist_wrapper.0.clone()));
        }

        // 查 LRU cache 和数据库
        self.inner.find_by_sort_name(sort_name).await
    }

    async fn delete(&self, artist_id: ArtistId) -> Result<(), ArtistError> {
        let id_i64 = artist_id.as_i64();

        self.memtable_context.delete(&id_i64).await.map_err(|e| {
            ArtistError::DbErr(format!("Failed to delete artist {}: {}", id_i64, e))
        })?;

        self.inner.delete(artist_id).await
    }
}
