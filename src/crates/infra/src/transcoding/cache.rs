use application::query::stream_cache::{StreamCache, StreamCacheData};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::PathBuf;

/// 缓存条目（存储在 sled 中）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    data: Vec<u8>,
    content_type: String,
    cache_key: String,
    size: u64,
    created_at: i64,
}

/// 基于 sled 的流媒体缓存实现
pub struct StreamCacheImpl {
    /// sled 数据库实例
    db: Db,
    /// 缓存过期时间（秒）
    ttl_secs: u64,
    /// 最大缓存文件大小（字节），超过此大小的文件不缓存
    max_cache_size: u64,
}

impl StreamCacheImpl {
    /// 创建新的流媒体缓存实例
    pub fn new(db_path: PathBuf, ttl_secs: u64) -> Result<Self, sled::Error> {
        // 确保目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let db = sled::open(db_path)?;

        Ok(Self {
            db,
            ttl_secs,
            max_cache_size: 100 * 1024 * 1024, // 100MB
        })
    }

    /// 设置最大缓存文件大小
    pub fn with_max_cache_size(mut self, size: u64) -> Self {
        self.max_cache_size = size;
        self
    }

    /// 检查缓存是否过期
    fn is_expired(&self, created_at: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        (now - created_at) > self.ttl_secs as i64
    }

    /// 从 sled 加载缓存
    fn load_from_db(&self, cache_key: &str) -> Option<StreamCacheData> {
        let value = self.db.get(cache_key.as_bytes()).ok()??;
        let entry: CacheEntry = serde_json::from_slice(&value).ok()?;

        // 检查是否过期
        if self.is_expired(entry.created_at) {
            // 删除过期条目
            let _ = self.db.remove(cache_key.as_bytes());
            log::debug!("Stream cache expired: {}", cache_key);
            return None;
        }

        log::debug!("Stream cache hit: {}", cache_key);
        Some(StreamCacheData {
            data: Bytes::from(entry.data),
            content_type: entry.content_type,
            cache_key: entry.cache_key,
            size: entry.size,
        })
    }

    /// 保存到 sled
    fn save_to_db(&self, cache_key: &str, data: &StreamCacheData) -> Result<(), sled::Error> {
        // 检查文件大小是否超过限制
        if data.size > self.max_cache_size {
            log::debug!(
                "Stream cache skip (too large): {} ({} bytes > {} bytes)",
                cache_key,
                data.size,
                self.max_cache_size
            );
            return Ok(());
        }

        let entry = CacheEntry {
            data: data.data.to_vec(),
            content_type: data.content_type.clone(),
            cache_key: data.cache_key.clone(),
            size: data.size,
            created_at: chrono::Utc::now().timestamp(),
        };

        let value = serde_json::to_vec(&entry).map_err(|e| {
            sled::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?;

        self.db.insert(cache_key.as_bytes(), value)?;
        log::debug!("Stream cache stored: {} ({} bytes)", cache_key, data.size);
        Ok(())
    }

    /// 从 sled 删除缓存
    fn remove_from_db(&self, cache_key: &str) {
        let _ = self.db.remove(cache_key.as_bytes());
        log::debug!("Stream cache invalidated: {}", cache_key);
    }

    /// 检查缓存是否存在（不加载数据）
    fn exists_in_db(&self, cache_key: &str) -> bool {
        match self.db.get(cache_key.as_bytes()) {
            Ok(Some(value)) => {
                // 检查是否过期
                if let Ok(entry) = serde_json::from_slice::<CacheEntry>(&value) {
                    if !self.is_expired(entry.created_at) {
                        return true;
                    }
                    // 过期则删除
                    let _ = self.db.remove(cache_key.as_bytes());
                }
                false
            }
            _ => false,
        }
    }
}

#[async_trait]
impl StreamCache for StreamCacheImpl {
    async fn get(&self, cache_key: &str) -> Option<StreamCacheData> {
        self.load_from_db(cache_key)
    }

    async fn put(&self, cache_key: &str, data: StreamCacheData) {
        if let Err(e) = self.save_to_db(cache_key, &data) {
            log::warn!("Failed to persist stream cache to sled: {}", e);
        }
    }

    async fn invalidate(&self, cache_key: &str) {
        self.remove_from_db(cache_key);
    }

    async fn exists(&self, cache_key: &str) -> bool {
        self.exists_in_db(cache_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_stream_cache_put_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let cache = StreamCacheImpl::new(temp_dir.path().to_path_buf(), 3600).unwrap();

        let data = StreamCacheData {
            data: Bytes::from_static(&[1, 2, 3, 4, 5]),
            content_type: "audio/mpeg".to_string(),
            cache_key: "test-audio".to_string(),
            size: 5,
        };

        cache.put("test-audio", data.clone()).await;

        let retrieved = cache.get("test-audio").await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(&retrieved.data[..], &[1, 2, 3, 4, 5]);
        assert_eq!(retrieved.content_type, "audio/mpeg");
    }

    #[tokio::test]
    async fn test_stream_cache_exists() {
        let temp_dir = TempDir::new().unwrap();
        let cache = StreamCacheImpl::new(temp_dir.path().to_path_buf(), 3600).unwrap();

        assert!(!cache.exists("non-existent").await);

        let data = StreamCacheData {
            data: Bytes::from_static(&[1, 2, 3]),
            content_type: "audio/flac".to_string(),
            cache_key: "exists-test".to_string(),
            size: 3,
        };

        cache.put("exists-test", data).await;
        assert!(cache.exists("exists-test").await);
    }

    #[tokio::test]
    async fn test_stream_cache_skip_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let cache = StreamCacheImpl::new(temp_dir.path().to_path_buf(), 3600)
            .unwrap()
            .with_max_cache_size(10); // 只允许 10 字节

        let data = StreamCacheData {
            data: Bytes::from(vec![0u8; 100]), // 100 字节
            content_type: "audio/mpeg".to_string(),
            cache_key: "large-file".to_string(),
            size: 100,
        };

        cache.put("large-file", data).await;
        // 应该不会被缓存
        assert!(cache.get("large-file").await.is_none());
    }
}
