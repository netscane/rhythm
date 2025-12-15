use crate::auth::AuthConfig;
use config::{Config, Environment, File};
use domain::cover_art::CoverSourceType;
use dotenvy::dotenv;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::RwLock;

/// 艺术家占位图文件名
const PLACE_HOLDER_ARTIST_ART: &str = "artist-placeholder.webp";
/// 专辑占位图文件名
const PLACE_HOLDER_ALBUM_ART: &str = "placeholder.png";

/// 查找资源文件，优先级：当前工作目录 > 可执行文件目录
fn find_resource_file(filename: &str) -> Option<String> {
    // 1. 优先从当前工作目录查找
    let cwd_path = std::env::current_dir()
        .ok()
        .map(|p| p.join("resources").join(filename));
    if let Some(ref path) = cwd_path {
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    // 2. 从可执行文件目录查找
    let exe_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .map(|p| p.join("resources").join(filename));
    if let Some(ref path) = exe_path {
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    None
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct RawConfig {
    auto_login_username: String,
    jwt_expire_secs: i64,
    jwt_secret_key: String,
    password_encryption_key: String,
    salt_cost: i32,
    ignoredarticles: String,
    indexgroups: String,
    database_url: String,
    cover_art_source_priority: HashMap<String, f32>,
    base_url: String,
    /// 封面文件名通配符列表（按优先级排序，越靠前优先级越高）
    cover_art_wildcards: Vec<String>,
    /// 缓存配置
    cache: RawCacheConfig,
    /// 服务器配置
    server: RawServerConfig,
    /// 转码配置
    transcoding: RawTranscodingConfig,
}

/// 缓存配置（原始配置）
#[derive(Debug, Deserialize)]
#[serde(default)]
struct RawCacheConfig {
    /// 缓存数据目录
    data_dir: String,
    /// 缓存过期时间（秒），默认 7 天
    ttl_secs: u64,
}

impl Default for RawCacheConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data/cache".to_string(),
            ttl_secs: 7 * 24 * 3600, // 7 天
        }
    }
}

/// 转码配置（原始配置）
#[derive(Debug, Deserialize)]
#[serde(default)]
struct RawTranscodingConfig {
    /// FFmpeg 可执行文件路径
    ffmpeg_path: String,
    /// 默认目标格式（当客户端未指定时使用）
    default_format: String,
    /// 默认比特率（kbps）
    default_bit_rate: i32,
    /// 是否启用转码缓存
    cache_enabled: bool,
    /// 转码缓存过期时间（秒），默认 30 天
    cache_ttl_secs: u64,
    /// 流式传输块大小（字节）
    chunk_size: usize,
    /// 无损格式列表（这些格式在请求时会被转码）
    lossless_formats: Vec<String>,
}

impl Default for RawTranscodingConfig {
    fn default() -> Self {
        Self {
            ffmpeg_path: "ffmpeg".to_string(),
            default_format: "mp3".to_string(),
            default_bit_rate: 192,
            cache_enabled: true,
            cache_ttl_secs: 30 * 24 * 3600, // 30 天
            chunk_size: 65536, // 64KB
            lossless_formats: vec![
                "flac".to_string(),
                "wav".to_string(),
                "aiff".to_string(),
                "ape".to_string(),
                "dsf".to_string(),
                "dff".to_string(),
                "wv".to_string(),
            ],
        }
    }
}

/// 服务器配置（原始配置）
#[derive(Debug, Deserialize)]
#[serde(default)]
struct RawServerConfig {
    /// 监听地址
    host: String,
    /// 监听端口
    port: u16,
}

impl Default for RawServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 5533,
        }
    }
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            auto_login_username: "".to_string(),
            jwt_expire_secs: 3600,
            salt_cost: 10,
            jwt_secret_key: "secret".to_string(),
            password_encryption_key: "default_password_encryption_key".to_string(),
            ignoredarticles: "The El La Los Las Le Les Os As O A".to_string(),
            indexgroups: "A B C D E F G H I J K L M N O P Q R S T U V W X-Z(XYZ) [Unknown]([)"
                .to_string(),
            database_url: "".to_string(),
            cover_art_source_priority: HashMap::from([
                ("embedded".to_string(), 15.0),
                ("external".to_string(), 10.0),
                ("downloaded".to_string(), 5.0),
                ("generated".to_string(), 2.0),
                ("manual".to_string(), 8.0),
            ]),
            base_url: "http://192.168.2.31:5533".to_string(),
            cover_art_wildcards: vec![
                "cover.*".to_string(),
                "folder.*".to_string(),
                "front.*".to_string(),
                "album.*".to_string(),
                "albumart.*".to_string(),
                "*".to_string(),
            ],
            cache: RawCacheConfig::default(),
            server: RawServerConfig::default(),
            transcoding: RawTranscodingConfig::default(),
        }
    }
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 缓存数据目录
    pub data_dir: String,
    /// 缓存过期时间（秒）
    pub ttl_secs: u64,
}

impl CacheConfig {
    /// 获取封面缓存目录路径
    pub fn cover_art_cache_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.data_dir).join("cover_art")
    }

    /// 获取音乐文件缓存目录路径（预留给后续使用）
    pub fn music_cache_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.data_dir).join("music")
    }
}

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 监听地址
    pub host: String,
    /// 监听端口
    pub port: u16,
}

/// 转码配置
#[derive(Debug, Clone)]
pub struct TranscodingConfig {
    /// FFmpeg 可执行文件路径
    pub ffmpeg_path: String,
    /// 默认目标格式（当客户端未指定时使用）
    pub default_format: String,
    /// 默认比特率（kbps）
    pub default_bit_rate: i32,
    /// 是否启用转码缓存
    pub cache_enabled: bool,
    /// 转码缓存过期时间（秒）
    pub cache_ttl_secs: u64,
    /// 流式传输块大小（字节）
    pub chunk_size: usize,
    /// 无损格式列表
    pub lossless_formats: Vec<String>,
}

impl TranscodingConfig {
    /// 检查格式是否为无损格式
    pub fn is_lossless(&self, format: &str) -> bool {
        self.lossless_formats
            .iter()
            .any(|f| f.eq_ignore_ascii_case(format))
    }

    /// 获取转码缓存目录路径
    pub fn cache_path(&self, cache_data_dir: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(cache_data_dir).join("transcoding")
    }
}

#[derive(Debug, Clone)]
pub struct AppConfigImpl {
    pub auto_login_username: Arc<RwLock<String>>,
    pub jwt_expire_secs: Arc<AtomicU64>,
    pub salt_cost: Arc<AtomicU64>,
    pub jwt_secret_key: Arc<RwLock<String>>,
    pub password_encryption_key: Arc<RwLock<String>>,
    pub ignoredarticles: Arc<RwLock<String>>,
    pub indexgroups: Arc<RwLock<String>>,
    pub database_url: Arc<RwLock<String>>,
    pub cover_art_source_priority: Arc<RwLock<HashMap<CoverSourceType, f32>>>,
    pub base_url: Arc<RwLock<String>>,
    pub cover_art_wildcards: Arc<RwLock<Vec<String>>>,
    pub cache: Arc<RwLock<CacheConfig>>,
    pub server: Arc<RwLock<ServerConfig>>,
    pub transcoding: Arc<RwLock<TranscodingConfig>>,
}

impl AppConfigImpl {
    fn new(data: RawConfig) -> Self {
        let cover_art_source_priority = data
            .cover_art_source_priority
            .iter()
            .map(|(k, v)| (CoverSourceType::from(k.as_str()), *v))
            .collect();
        let cache_config = CacheConfig {
            data_dir: data.cache.data_dir,
            ttl_secs: data.cache.ttl_secs,
        };
        let server_config = ServerConfig {
            host: data.server.host,
            port: data.server.port,
        };
        let transcoding_config = TranscodingConfig {
            ffmpeg_path: data.transcoding.ffmpeg_path,
            default_format: data.transcoding.default_format,
            default_bit_rate: data.transcoding.default_bit_rate,
            cache_enabled: data.transcoding.cache_enabled,
            cache_ttl_secs: data.transcoding.cache_ttl_secs,
            chunk_size: data.transcoding.chunk_size,
            lossless_formats: data.transcoding.lossless_formats,
        };
        AppConfigImpl {
            auto_login_username: Arc::new(RwLock::new(data.auto_login_username)),
            jwt_expire_secs: Arc::new(AtomicU64::new(data.jwt_expire_secs as u64)),
            salt_cost: Arc::new(AtomicU64::new(data.salt_cost as u64)),
            jwt_secret_key: Arc::new(RwLock::new(data.jwt_secret_key)),
            password_encryption_key: Arc::new(RwLock::new(data.password_encryption_key)),
            ignoredarticles: Arc::new(RwLock::new(data.ignoredarticles)),
            indexgroups: Arc::new(RwLock::new(data.indexgroups)),
            database_url: Arc::new(RwLock::new(data.database_url)),
            cover_art_source_priority: Arc::new(RwLock::new(cover_art_source_priority)),
            base_url: Arc::new(RwLock::new(data.base_url)),
            cover_art_wildcards: Arc::new(RwLock::new(data.cover_art_wildcards)),
            cache: Arc::new(RwLock::new(cache_config)),
            server: Arc::new(RwLock::new(server_config)),
            transcoding: Arc::new(RwLock::new(transcoding_config)),
        }
    }

    pub fn cover_art_wildcards(&self) -> Vec<String> {
        let cfg_val = self.cover_art_wildcards.read().unwrap();
        cfg_val.clone()
    }

    pub fn cache(&self) -> CacheConfig {
        let cfg_val = self.cache.read().unwrap();
        cfg_val.clone()
    }

    pub fn server(&self) -> ServerConfig {
        let cfg_val = self.server.read().unwrap();
        cfg_val.clone()
    }

    pub fn transcoding(&self) -> TranscodingConfig {
        let cfg_val = self.transcoding.read().unwrap();
        cfg_val.clone()
    }

    pub fn load() -> Result<AppConfigImpl, Box<dyn Error>> {
        dotenv().ok();

        let config = Config::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        let raw: RawConfig = config.try_deserialize()?; // serde 自动填充默认值
        let app_config = AppConfigImpl::new(raw);
        Ok(app_config)
    }

    pub fn indexgroups(&self) -> String {
        let cfg_val = self.indexgroups.read().unwrap();
        (*cfg_val).clone()
    }

    pub fn ignored_articles(&self) -> &[String] {
        static CACHED_ARTICLES: OnceLock<Vec<String>> = OnceLock::new();

        let articles_string = {
            let cfg_val = self.ignoredarticles.read().unwrap();
            cfg_val.clone()
        };

        CACHED_ARTICLES.get_or_init(|| {
            articles_string
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        })
    }
    pub fn database_url(&self) -> String {
        let cfg_val = self.database_url.read().unwrap();
        (*cfg_val).clone()
    }

    pub fn base_url(&self) -> String {
        let cfg_val = self.base_url.read().unwrap();
        (*cfg_val).clone()
    }

    pub fn password_encryption_key(&self) -> String {
        let cfg_val = self.password_encryption_key.read().unwrap();
        (*cfg_val).clone()
    }
}

impl AuthConfig for AppConfigImpl {
    fn jwt_secret(&self) -> &str {
        static CACHED_SECRET: OnceLock<String> = OnceLock::new();

        let secret_string = {
            let cfg_val = self.jwt_secret_key.read().unwrap();
            cfg_val.clone()
        };

        CACHED_SECRET.get_or_init(|| secret_string)
    }

    fn jwt_expire_secs(&self) -> i64 {
        self.jwt_expire_secs.load(Ordering::SeqCst) as i64
    }

    fn salt_cost(&self) -> i32 {
        self.salt_cost.load(Ordering::SeqCst) as i32
    }
}

impl application::query::config::CoverArtConfig for AppConfigImpl {
    fn cover_art_wildcards(&self) -> Vec<String> {
        self.cover_art_wildcards()
    }

    fn artist_placeholder_path(&self) -> Option<String> {
        find_resource_file(PLACE_HOLDER_ARTIST_ART)
    }

    fn album_placeholder_path(&self) -> Option<String> {
        find_resource_file(PLACE_HOLDER_ALBUM_ART)
    }
}
