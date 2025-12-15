use application::query::get_cover_art::{CoverArtCache, CoverArtData};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::PathBuf;

/// 缓存条目（存储在 sled 中）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    data: Vec<u8>,
    mime_type: String,
    cache_key: String,
    last_modified: i64,
    created_at: i64,
}

/// 基于 sled 的持久化封面艺术缓存实现
/// sled 内部已有内存缓存，无需额外的内存层
pub struct CoverArtCacheImpl {
    /// sled 数据库实例
    db: Db,
    /// 缓存过期时间（秒）
    ttl_secs: u64,
}

impl CoverArtCacheImpl {
    /// 创建新的持久化缓存实例
    ///
    /// # Arguments
    /// * `db_path` - sled 数据库路径
    /// * `ttl_secs` - 缓存过期时间（秒）
    pub fn new(db_path: PathBuf, ttl_secs: u64) -> Result<Self, sled::Error> {
        let db = sled::open(db_path)?;

        Ok(Self { db, ttl_secs })
    }

    /// 使用默认配置创建缓存（7 天过期）
    pub fn with_defaults(db_path: PathBuf) -> Result<Self, sled::Error> {
        Self::new(db_path, 7 * 24 * 3600)
    }

    /// 检查缓存是否过期
    fn is_expired(&self, created_at: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        (now - created_at) > self.ttl_secs as i64
    }

    /// 从 sled 加载缓存
    fn load_from_db(&self, cache_key: &str) -> Option<CoverArtData> {
        let value = self.db.get(cache_key.as_bytes()).ok()??;
        let entry: CacheEntry = serde_json::from_slice(&value).ok()?;

        // 检查是否过期
        if self.is_expired(entry.created_at) {
            // 删除过期条目
            let _ = self.db.remove(cache_key.as_bytes());
            return None;
        }

        Some(CoverArtData {
            data: Bytes::from(entry.data),
            mime_type: entry.mime_type,
            cache_key: entry.cache_key,
            last_modified: entry.last_modified,
        })
    }

    /// 保存到 sled
    fn save_to_db(&self, cache_key: &str, data: &CoverArtData) -> Result<(), sled::Error> {
        let entry = CacheEntry {
            data: data.data.to_vec(),
            mime_type: data.mime_type.clone(),
            cache_key: data.cache_key.clone(),
            last_modified: data.last_modified,
            created_at: chrono::Utc::now().timestamp(),
        };

        let value = serde_json::to_vec(&entry).map_err(|e| {
            sled::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?;

        self.db.insert(cache_key.as_bytes(), value)?;
        Ok(())
    }

    /// 从 sled 删除缓存
    fn remove_from_db(&self, cache_key: &str) {
        let _ = self.db.remove(cache_key.as_bytes());
    }
}

#[async_trait]
impl CoverArtCache for CoverArtCacheImpl {
    async fn get(&self, cache_key: &str) -> Option<CoverArtData> {
        self.load_from_db(cache_key)
    }

    async fn put(&self, cache_key: &str, data: CoverArtData) {
        if let Err(e) = self.save_to_db(cache_key, &data) {
            log::warn!("Failed to persist cover art cache to sled: {}", e);
        }
    }

    async fn invalidate(&self, cache_key: &str) {
        self.remove_from_db(cache_key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let cache = CoverArtCacheImpl::new(temp_dir.path().to_path_buf(), 3600).unwrap();

        let data = CoverArtData {
            data: Bytes::from_static(&[1, 2, 3, 4]),
            mime_type: "image/jpeg".to_string(),
            cache_key: "test-key".to_string(),
            last_modified: 12345,
        };

        cache.put("test-key", data.clone()).await;

        let retrieved = cache.get("test-key").await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(&retrieved.data[..], &[1, 2, 3, 4]);
        assert_eq!(retrieved.mime_type, "image/jpeg");
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let temp_dir = TempDir::new().unwrap();
        let cache = CoverArtCacheImpl::new(temp_dir.path().to_path_buf(), 3600).unwrap();

        let result = cache.get("non-existent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let temp_dir = TempDir::new().unwrap();
        let cache = CoverArtCacheImpl::new(temp_dir.path().to_path_buf(), 3600).unwrap();

        let data = CoverArtData {
            data: Bytes::from_static(&[1, 2, 3]),
            mime_type: "image/png".to_string(),
            cache_key: "invalidate-test".to_string(),
            last_modified: 12345,
        };

        cache.put("invalidate-test", data).await;
        assert!(cache.get("invalidate-test").await.is_some());

        cache.invalidate("invalidate-test").await;
        assert!(cache.get("invalidate-test").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_survives_restart() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().to_path_buf();

        // 创建缓存并写入数据
        {
            let cache = CoverArtCacheImpl::new(db_path.clone(), 3600).unwrap();

            let data = CoverArtData {
                data: Bytes::from_static(&[5, 6, 7, 8]),
                mime_type: "image/png".to_string(),
                cache_key: "persist-test".to_string(),
                last_modified: 67890,
            };

            cache.put("persist-test", data).await;
        }

        // 创建新的缓存实例（模拟应用重启）
        let cache = CoverArtCacheImpl::new(db_path, 3600).unwrap();

        // 应该能从 sled 加载
        let retrieved = cache.get("persist-test").await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(&retrieved.data[..], &[5, 6, 7, 8]);
        assert_eq!(retrieved.mime_type, "image/png");
    }
}
