use async_trait::async_trait;
use bytes::Bytes;

/// 流媒体缓存数据
#[derive(Debug, Clone)]
pub struct StreamCacheData {
    /// 音频数据
    pub data: Bytes,
    /// MIME 类型
    pub content_type: String,
    /// 缓存键
    pub cache_key: String,
    /// 文件大小
    pub size: u64,
}

/// 流媒体缓存配置 trait
#[async_trait]
pub trait StreamCacheConfig: Send + Sync {
    /// 是否启用缓存
    fn cache_enabled(&self) -> bool;
    /// 默认转码格式
    fn default_format(&self) -> String;
    /// 默认比特率（kbps）
    fn default_bit_rate(&self) -> i32;
    /// 是否为无损格式
    fn is_lossless(&self, format: &str) -> bool;
}

/// 流媒体缓存 trait
#[async_trait]
pub trait StreamCache: Send + Sync {
    /// 获取缓存的流数据
    async fn get(&self, cache_key: &str) -> Option<StreamCacheData>;

    /// 存储流数据到缓存
    async fn put(&self, cache_key: &str, data: StreamCacheData);

    /// 使缓存失效
    async fn invalidate(&self, cache_key: &str);

    /// 检查缓存是否存在
    async fn exists(&self, cache_key: &str) -> bool;
}

/// 生成缓存键
/// 
/// 缓存键格式：`{audio_file_id}_{format}_{bit_rate}`
/// - 原始文件：`{audio_file_id}_raw_0`
/// - 转码文件：`{audio_file_id}_{format}_{bit_rate}`
pub fn generate_cache_key(audio_file_id: i64, format: &str, bit_rate: i32) -> String {
    format!("{}_{}_{}",audio_file_id, format.to_lowercase(), bit_rate)
}

/// 生成原始文件缓存键
pub fn generate_raw_cache_key(audio_file_id: i64, suffix: &str) -> String {
    format!("{}_raw_{}", audio_file_id, suffix.to_lowercase())
}
