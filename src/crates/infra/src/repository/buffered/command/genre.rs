use async_trait::async_trait;
use domain::genre::{Genre, GenreError, GenreName, GenreRepository};
use domain::value::GenreId;
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
struct GenreWrapper(Genre);

// 为 Genre 实现 MemtableValue
impl MemtableValue<i64> for GenreWrapper {
    fn get_key(&self) -> i64 {
        self.0.id.as_i64()
    }

    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)> {
        vec![
            ("name", IndexValue::String(self.0.name.value().clone()), IndexMatch::Exact),
        ]
    }

    fn get_index(&self, index_name: &str) -> IndexValue {
        match index_name {
            "name" => IndexValue::String(self.0.name.value().clone()),
            _ => panic!("Invalid index name: {}", index_name),
        }
    }
}

pub struct LruCacheGenreRepository<R>
where
    R: GenreRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
    lru_cache: Arc<RwLock<LruCache<i64, Genre>>>,
    lru_name_index: Arc<RwLock<HashMap<String, i64>>>,
}

impl<R> LruCacheGenreRepository<R>
where
    R: GenreRepository + Send + Sync + 'static,
{
    pub fn new(inner: Arc<R>, cache_capacity: usize) -> Arc<Self> {
        let lru_capacity = NonZeroUsize::new(cache_capacity).unwrap();
        let lru_cache = Arc::new(RwLock::new(LruCache::new(lru_capacity)));
        let lru_name_index = Arc::new(RwLock::new(HashMap::new()));
        Arc::new(Self {
            inner,
            lru_cache,
            lru_name_index,
        })
    }
}

#[async_trait]
impl<R> GenreRepository for LruCacheGenreRepository<R>
where
    R: GenreRepository + Send + Sync + 'static,
{
    async fn save(&self, genre: Genre) -> Result<Genre, GenreError> {
        let result = self.inner.save(genre).await?;

        // 缓存结果
        let id_i64 = result.id.as_i64();
        let name = result.name.value().clone();
        
        // ⚠️ 修复死锁：分开获取两个写锁
        {
            let mut lru_cache = self.lru_cache.write().await;
            lru_cache.put(id_i64, result.clone());
        }
        {
            let mut lru_name_index = self.lru_name_index.write().await;
            lru_name_index.insert(name, id_i64);
        }
        
        Ok(result)
    }

    async fn find_by_id(&self, id: GenreId) -> Result<Option<Genre>, GenreError> {
        let id_i64 = id.as_i64();

        // 先查 LRU cache
        {
            let mut lru_cache = self.lru_cache.write().await;
            if let Some(genre) = lru_cache.get(&id_i64) {
                return Ok(Some(genre.clone()));
            }
        }

        // 查数据库
        match self.inner.find_by_id(id).await {
            Ok(Some(genre)) => {
                let id_i64 = genre.id.as_i64();
                let name = genre.name.value().clone();
                
                // ⚠️ 修复死锁：分开获取两个写锁
                {
                    let mut lru_cache = self.lru_cache.write().await;
                    lru_cache.put(id_i64, genre.clone());
                }
                {
                    let mut lru_name_index = self.lru_name_index.write().await;
                    lru_name_index.insert(name, id_i64);
                }
                
                Ok(Some(genre))
            }
            result => result,
        }
    }

    async fn find_by_name(&self, genre_name: &GenreName) -> Result<Option<Genre>, GenreError> {
        let name = genre_name.value();

        // 先查 LRU cache
        {
            let lru_name_index = self.lru_name_index.read().await;
            if let Some(id) = lru_name_index.get(&name) {
                let mut lru_cache = self.lru_cache.write().await;
                if let Some(genre) = lru_cache.get(id) {
                    return Ok(Some(genre.clone()));
                }
            }
        }

        // 查数据库并缓存结果
        match self.inner.find_by_name(genre_name).await {
            Ok(Some(genre)) => {
                let id_i64 = genre.id.as_i64();
                let name = genre.name.value().clone();
                
                // ⚠️ 修复死锁：分开获取两个写锁
                {
                    let mut lru_cache = self.lru_cache.write().await;
                    lru_cache.put(id_i64, genre.clone());
                }
                {
                    let mut lru_name_index = self.lru_name_index.write().await;
                    lru_name_index.insert(name, id_i64);
                }
                
                Ok(Some(genre))
            }
            result => result,
        }
    }

    async fn delete(&self, genre_id: GenreId) -> Result<(), GenreError> {
        let id_i64 = genre_id.as_i64();

        // 从 LRU cache 中移除
        // ⚠️ 修复死锁：先获取值，然后分开获取两个写锁
        let name_opt = {
            let mut lru_cache = self.lru_cache.write().await;
            lru_cache.pop(&id_i64).map(|genre| genre.name.value().clone())
        };
        
        if let Some(name) = name_opt {
            let mut lru_name_index = self.lru_name_index.write().await;
            lru_name_index.remove(&name);
        }

        // 从数据库删除
        self.inner.delete(genre_id).await
    }
}

// Genre Persister：实现 MemtablePersister trait
pub struct GenrePersister<R>
where
    R: GenreRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
    save_semaphore: Arc<Semaphore>,
    concurrency: usize,
}

impl<R> Clone for GenrePersister<R>
where
    R: GenreRepository + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            save_semaphore: self.save_semaphore.clone(),
            concurrency: self.concurrency,
        }
    }
}

impl<R> GenrePersister<R>
where
    R: GenreRepository + Send + Sync + 'static,
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
impl<R> MemtablePersister<i64, GenreWrapper> for GenrePersister<R>
where
    R: GenreRepository + Send + Sync + 'static,
{
    async fn persist(&self, key: i64, value: Arc<GenreWrapper>) -> Result<(), String> {
        let _permit = self
            .save_semaphore
            .acquire()
            .await
            .map_err(|e| format!("Failed to acquire semaphore: {}", e))?;

        let mut genre = value.0.clone();
        genre.version -= 1;

        // 重试机制
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            match self.inner.save(genre.clone()).await {
                Ok(_) => {
                    //info!("Successfully saved genre {} to database", key);
                    return Ok(());
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    
                    // 版本冲突不是错误：表示数据已经被成功保存过了
                    if error_msg.contains("Version conflict") {
                        info!("Genre {} already saved (version conflict), skipping", key);
                        return Ok(());
                    }
                    
                    // 连接池超时错误：进行重试
                    if error_msg.contains("Connection pool timed out")
                        || error_msg.contains("pool timed out")
                    {
                        if retry_count < MAX_RETRIES {
                            let delay_ms = 20 * (1 << retry_count);
                            warn!(
                                "Connection pool timeout for genre {}, retrying in {}ms (attempt {}/{})",
                                key, delay_ms, retry_count + 1, MAX_RETRIES
                            );
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            retry_count += 1;
                            continue;
                        }
                    }
                    
                    // 其他错误：记录并返回错误
                    error!("Failed to save genre {} to database: {}", key, e);
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

        let genre_id = GenreId::from(key);

        self.inner
            .delete(genre_id)
            .await
            .map_err(|e| format!("Failed to delete: {}", e))?;

        info!("Successfully deleted genre {} from database", key);
        Ok(())
    }

    async fn persist_batch(&self, items: Vec<(i64, Arc<GenreWrapper>)>) -> Result<(), String> {
        let start_time = Instant::now();
        info!("Starting batch persist for {} genres", items.len());

        stream::iter(items)
            .map(|(key, value)| {
                let persister = self.clone();
                async move {
                    if let Err(e) = persister.persist(key, value).await {
                        error!("Failed to persist genre {}: {}", key, e);
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

pub struct BufferedGenreRepository<R>
where
    R: GenreRepository + Send + Sync + 'static,
{
    memtable_context:
        Arc<MemtableContext<i64, GenreWrapper, GenrePersister<LruCacheGenreRepository<R>>>>,
    inner: Arc<LruCacheGenreRepository<R>>,
}

impl<R> BufferedGenreRepository<R>
where
    R: GenreRepository + Send + Sync + 'static,
{
    pub fn new(
        inner: R,
        cache_capacity: usize,
        concurrency: usize,
        flush_timeout: Duration,
    ) -> Arc<Self> {
        let memtable_size_threshold = cache_capacity.max(100);

        // 创建 persister
        let inner_arc = LruCacheGenreRepository::new(Arc::new(inner), cache_capacity);
        let persister = Arc::new(GenrePersister::new(inner_arc.clone(), concurrency));

        let active_memtable = Arc::new(RwLock::new(Memtable::<i64, GenreWrapper>::new()));
        let active_size = Arc::new(AtomicUsize::new(0));

        let memtable_context = Arc::new(MemtableContext::new(
            "Genre".to_string(),
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
        info!("Starting graceful shutdown of BufferedGenreRepository");

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
impl<R> GenreRepository for BufferedGenreRepository<R>
where
    R: GenreRepository + Send + Sync + 'static,
{
    async fn save(&self, mut genre: Genre) -> Result<Genre, GenreError> {
        genre.version += 1;
        let id_i64 = genre.id.as_i64();
        let genre_wrapper = Arc::new(GenreWrapper(genre.clone()));

        self.memtable_context
            .insert(id_i64, genre_wrapper)
            .await
            .map_err(|e| GenreError::DbErr(format!("Failed to insert into memtable: {}", e)))?;

        Ok(genre)
    }

    async fn find_by_id(&self, id: GenreId) -> Result<Option<Genre>, GenreError> {
        let id_i64 = id.as_i64();

        // 先查 active memtable
        if let Some(genre_wrapper) = self.memtable_context.get(&id_i64).await {
            return Ok(Some(genre_wrapper.0.clone()));
        }

        // 查 LRU cache 和数据库
        self.inner.find_by_id(id).await
    }

    async fn find_by_name(&self, genre_name: &GenreName) -> Result<Option<Genre>, GenreError> {
        let name = genre_name.value().clone();

        // 先查 active memtable 按名称查询
        if let Some(genre_wrapper) = self
            .memtable_context
            .get_by_index("name", IndexValue::String(name))
            .await
        {
            return Ok(Some(genre_wrapper.0.clone()));
        }

        // 查 LRU cache 和数据库
        self.inner.find_by_name(genre_name).await
    }

    async fn delete(&self, genre_id: GenreId) -> Result<(), GenreError> {
        let id_i64 = genre_id.as_i64();

        self.memtable_context
            .delete(&id_i64)
            .await
            .map_err(|e| GenreError::DbErr(format!("Failed to delete genre {}: {}", id_i64, e)))?;

        self.inner.delete(genre_id).await
    }
}
