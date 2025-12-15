use async_trait::async_trait;
use domain::audio_file::{AudioFile, AudioFileError, AudioFileRepository};
use domain::value::{AudioFileId, MediaPath};
use futures::stream::{self, StreamExt};
use log::{error, info, warn};
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};

// 导入 memtable 模块
use super::super::memtable::{
    IndexValue, IndexMatch, Memtable, MemtableContext, MemtablePersister, MemtableValue,
};

#[derive(Clone)]
struct AudioFileWrapper(AudioFile);

// 为 AudioFile 实现 MemtableValue
impl MemtableValue<i64> for AudioFileWrapper {
    fn get_key(&self) -> i64 {
        self.0.id.as_i64()
    }

    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)> {
        vec![
            ("path", IndexValue::String(format!("{}:{}", self.0.path.protocol, self.0.path.path)), IndexMatch::Exact),
        ]
    }
    
    fn get_index(&self, index_name: &str) -> IndexValue {
        match index_name {
            "path" => IndexValue::String(format!("{}:{}", self.0.path.protocol, self.0.path.path)),
            _ => panic!("Invalid index name: {}", index_name),
        }
    }
}

pub struct LruCacheAudioFileRepository<R>
where
    R: AudioFileRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
    lru_cache: Arc<RwLock<LruCache<i64, AudioFile>>>,
    lru_path_index: Arc<RwLock<HashMap<String, i64>>>,
}

impl<R> LruCacheAudioFileRepository<R>
where
    R: AudioFileRepository + Send + Sync + 'static,
{
    pub fn new(inner: Arc<R>, cache_capacity: usize) -> Arc<Self> {
        let lru_capacity = NonZeroUsize::new(cache_capacity).unwrap();
        let lru_cache = Arc::new(RwLock::new(LruCache::new(lru_capacity)));
        let lru_path_index = Arc::new(RwLock::new(HashMap::new()));
        Arc::new(Self {
            inner,
            lru_cache,
            lru_path_index,
        })
    }
}

#[async_trait]
impl<R> AudioFileRepository for LruCacheAudioFileRepository<R>
where
    R: AudioFileRepository + Send + Sync + 'static,
{
    async fn save(&self, audio_file: AudioFile) -> Result<AudioFile, AudioFileError> {
        // 保存到数据库
        let af = self.inner.save(audio_file).await?;

        // 缓存结果（保存成功后的数据可能被数据库层修改，如版本号）
        let id_i64 = af.id.as_i64();
        let path_key = format!("{}:{}", af.path.protocol, af.path.path);
        {
            let mut lru_cache = self.lru_cache.write().await;
            lru_cache.put(id_i64, af.clone());
            let mut lru_path_index = self.lru_path_index.write().await;
            lru_path_index.insert(path_key, id_i64);
        }
        Ok(af)
    }

    async fn find_by_id(&self, id: &AudioFileId) -> Result<Option<AudioFile>, AudioFileError> {
        let id_i64 = id.as_i64();

        // 1. 先查 LRU cache（需要 write lock，因为 get 会更新 LRU 顺序）
        {
            let mut lru_cache = self.lru_cache.write().await;
            if let Some(audio) = lru_cache.get(&id_i64) {
                return Ok(Some(audio.clone()));
            }
        }

        // 2. 查数据库
        match self.inner.find_by_id(id).await {
            Ok(Some(audio)) => {
                // 将结果放入 LRU cache
                let id_i64 = audio.id.as_i64();
                let path_key = format!("{}:{}", audio.path.protocol, audio.path.path);
                {
                    let mut lru_cache = self.lru_cache.write().await;
                    lru_cache.put(id_i64, audio.clone());
                    let mut lru_path_index = self.lru_path_index.write().await;
                    lru_path_index.insert(path_key, id_i64);
                }
                Ok(Some(audio))
            }
            result => result,
        }
    }

    async fn find_by_path(&self, path: &MediaPath) -> Result<Option<AudioFile>, AudioFileError> {
        let path_key = format!("{}:{}", path.protocol, path.path);

        // 1. 先查 LRU cache（通过 path_index）
        {
            let lru_path_index = self.lru_path_index.read().await;
            if let Some(id) = lru_path_index.get(&path_key) {
                let mut lru_cache = self.lru_cache.write().await;
                if let Some(audio) = lru_cache.get(id) {
                    return Ok(Some(audio.clone()));
                }
            }
        }

        // 2. 查数据库并缓存结果
        match self.inner.find_by_path(path).await {
            Ok(Some(audio)) => {
                let id_i64 = audio.id.as_i64();
                let path_key = format!("{}:{}", audio.path.protocol, audio.path.path);
                {
                    let mut lru_cache = self.lru_cache.write().await;
                    lru_cache.put(id_i64, audio.clone());
                    let mut lru_path_index = self.lru_path_index.write().await;
                    lru_path_index.insert(path_key, id_i64);
                }
                Ok(Some(audio))
            }
            result => result,
        }
    }

    async fn delete(&self, id: &AudioFileId) -> Result<(), AudioFileError> {
        let id_i64 = id.as_i64();

        // 从 LRU cache 中移除
        {
            let mut lru_cache = self.lru_cache.write().await;
            if let Some(audio_file) = lru_cache.pop(&id_i64) {
                let mut lru_path_index = self.lru_path_index.write().await;
                let path_key = format!("{}:{}", audio_file.path.protocol, audio_file.path.path);
                lru_path_index.remove(&path_key);
            }
        }

        // 从数据库删除
        self.inner.delete(id).await
    }
}

// AudioFile Persister：实现 MemtablePersister trait
pub struct AudioFilePersister<R>
where
    R: AudioFileRepository + Send + Sync + 'static,
{
    inner: Arc<R>,
    save_semaphore: Arc<Semaphore>,
    concurrency: usize,
}

// 手动实现 Clone（因为内部都是 Arc）
impl<R> Clone for AudioFilePersister<R>
where
    R: AudioFileRepository + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            save_semaphore: self.save_semaphore.clone(),
            concurrency: self.concurrency,
        }
    }
}

impl<R> AudioFilePersister<R>
where
    R: AudioFileRepository + Send + Sync + 'static,
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
impl<R> MemtablePersister<i64, AudioFileWrapper> for AudioFilePersister<R>
where
    R: AudioFileRepository + Send + Sync + 'static,
{
    async fn persist(&self, key: i64, value: Arc<AudioFileWrapper>) -> Result<(), String> {
        let _permit = self
            .save_semaphore
            .acquire()
            .await
            .map_err(|e| format!("Failed to acquire semaphore: {}", e))?;

        let audio_file = value.0.clone();

        // 重试机制
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            let mut af = audio_file.clone();
            af.version -= 1;
            match self.inner.save(af).await {
                Ok(_) => {
                    //info!("Successfully saved audio file {} to database", key);
                    return Ok(());
                }
                Err(e) => {
                    let error_msg = format!("{}", e);

                    // 版本冲突不是错误：表示数据已经被成功保存过了
                    if error_msg.contains("Version conflict") {
                        info!(
                            "Audio file {} already saved (version conflict), skipping",
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
                                "Connection pool timeout for audio file {}, retrying in {}ms (attempt {}/{})",
                                key, delay_ms, retry_count + 1, MAX_RETRIES
                            );
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            retry_count += 1;
                            continue;
                        }
                    }

                    // 其他错误：记录并返回错误
                    error!("Failed to save audio file {} to database: {}", key, e);
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

        let audio_file_id = domain::value::AudioFileId::from(key);

        self.inner
            .delete(&audio_file_id)
            .await
            .map_err(|e| format!("Failed to delete: {}", e))?;

        info!("Successfully deleted audio file {} from database", key);
        Ok(())
    }

    async fn persist_batch(&self, items: Vec<(i64, Arc<AudioFileWrapper>)>) -> Result<(), String> {
        let start_time = Instant::now();
        info!("Starting batch persist for {} audio files", items.len());

        stream::iter(items)
            .map(|(key, value)| {
                let persister = self.clone();
                async move {
                    if let Err(e) = persister.persist(key, value).await {
                        error!("Failed to persist audio file {}: {}", key, e);
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
        // 异步并发删除
        let mut handles = vec![];

        for key in keys {
            let persister = self.clone();
            let handle = tokio::spawn(async move { persister.remove(key).await });
            handles.push(handle);
        }

        // 等待所有任务完成
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Task failed: {}", e);
            }
        }

        Ok(())
    }
}

pub struct BufferedAudioFileRepository<R>
where
    R: AudioFileRepository + Send + Sync + 'static,
{
    memtable_context: Arc<
        MemtableContext<i64, AudioFileWrapper, AudioFilePersister<LruCacheAudioFileRepository<R>>>,
    >,
    inner: Arc<LruCacheAudioFileRepository<R>>,
}

impl<R> BufferedAudioFileRepository<R>
where
    R: AudioFileRepository + Send + Sync + 'static,
{
    pub fn new(
        inner: R,
        cache_capacity: usize,
        concurrency: usize,
        flush_timeout: Duration,
    ) -> Arc<Self> {
        let memtable_size_threshold = cache_capacity.max(100);

        // 创建 persister
        let inner_arc = LruCacheAudioFileRepository::new(Arc::new(inner), cache_capacity);
        let persister = Arc::new(AudioFilePersister::new(inner_arc.clone(), concurrency));

        // 创建 memtable 和相关组件
        let active_memtable = Arc::new(RwLock::new(Memtable::<i64, AudioFileWrapper>::new()));
        let active_size = Arc::new(AtomicUsize::new(0));

        let memtable_context = Arc::new(MemtableContext::new(
            "AudioFile".to_string(),
            active_memtable.clone(),
            active_size.clone(),
            memtable_size_threshold,
            persister,
            flush_timeout,
        ));

        // 启动自动 flush 定时器（由 MemtableContext 自己管理）
        memtable_context.start_auto_flush_timer();

        Arc::new(Self {
            memtable_context,
            inner: inner_arc.clone(),
        })
    }

    /// 优雅关闭：flush 所有待持久化的数据
    ///
    /// 应该在应用关闭时调用，确保所有数据都持久化到数据库
    ///
    /// 这个方法会：
    /// 1. 触发 active memtable 的强制旋转
    /// 2. 等待一段时间让异步 flush 任务完成
    ///
    /// # Arguments
    /// * `wait_duration` - 等待 flush 完成的最大时间
    ///
    /// # Returns
    /// * `Ok(Some(count))` - 成功触发 flush 的数据条数
    /// * `Ok(None)` - 没有数据需要 flush
    ///
    /// # Example
    /// ```rust
    /// // 在应用关闭时调用
    /// match repo.shutdown_gracefully(Duration::from_secs(30)).await {
    ///     Ok(Some(count)) => println!("Flushed {} items", count),
    ///     Ok(None) => println!("No data to flush"),
    ///     Err(e) => eprintln!("Shutdown error: {}", e),
    /// }
    /// ```
    pub async fn shutdown_gracefully(
        &self,
        wait_duration: Duration,
    ) -> Result<Option<usize>, String> {
        info!("Starting graceful shutdown of BufferedAudioFileRepository");

        // 1. 触发 memtable 强制 flush
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

        // 2. 等待异步 flush 任务完成
        tokio::time::sleep(wait_duration).await;

        info!(
            "Graceful shutdown completed (waited {:?} for flush tasks)",
            wait_duration
        );
        Ok(Some(count))
    }
}

#[async_trait]
impl<R> AudioFileRepository for BufferedAudioFileRepository<R>
where
    R: AudioFileRepository + Send + Sync + 'static,
{
    async fn save(&self, mut audio_file: AudioFile) -> Result<AudioFile, AudioFileError> {
        // 增加版本号
        audio_file.version += 1;
        let id_i64 = audio_file.id.as_i64();
        let audio_file_wrapper = Arc::new(AudioFileWrapper(audio_file.clone()));

        // 使用封装的 insert 方法（自动处理计数和旋转）
        self.memtable_context
            .insert(id_i64, audio_file_wrapper)
            .await
            .map_err(|e| {
                AudioFileError::DbError(format!("Failed to insert into memtable: {}", e))
            })?;

        Ok(audio_file)
    }

    async fn find_by_id(&self, id: &AudioFileId) -> Result<Option<AudioFile>, AudioFileError> {
        let id_i64 = id.as_i64();

        // 1. 先查 active memtable（使用封装方法）
        if let Some(audio_file_wrapper) = self.memtable_context.get(&id_i64).await {
            return Ok(Some(audio_file_wrapper.0.clone()));
        }

        // 2. 查 LRU cache 和数据库（通过 inner，inner 是 LruCacheAudioFileRepository）
        self.inner.find_by_id(id).await
    }

    async fn find_by_path(&self, path: &MediaPath) -> Result<Option<AudioFile>, AudioFileError> {
        let path_key = format!("{}:{}", path.protocol, path.path);

        // 1. 先查 active memtable 按路径查询（使用封装方法）
        if let Some(audio_file_wrapper) = self
            .memtable_context
            .get_by_index("path", IndexValue::String(path_key))
            .await
        {
            return Ok(Some(audio_file_wrapper.0.clone()));
        }

        // 2. 查 LRU cache 和数据库（通过 inner，inner 是 LruCacheAudioFileRepository）
        self.inner.find_by_path(path).await
    }

    async fn delete(&self, id: &AudioFileId) -> Result<(), AudioFileError> {
        let id_i64 = id.as_i64();

        // 从 active memtable 中添加 tombstone
        self.memtable_context.delete(&id_i64).await.map_err(|e| {
            AudioFileError::DbError(format!("Failed to delete audio file {}: {}", id_i64, e))
        })?;

        // 从 LRU cache 和数据库删除（通过 inner，inner 是 LruCacheAudioFileRepository）
        self.inner.delete(id).await
    }
}
