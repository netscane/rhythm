use crate::query::config::CoverArtConfig;
use crate::query::dao::{CoverArtDao, CoverArtPath};
use crate::query::dto::artwork_id::{ArtworkId, ArtworkKind};
use crate::query::QueryError;
use async_trait::async_trait;
use bytes::Bytes;
use image::ImageFormat;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

/// 1x1 透明 PNG（最终回退）
const FALLBACK_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// 封面艺术数据（包含图片数据和元信息）
/// 使用 Bytes 实现零拷贝共享
#[derive(Debug, Clone)]
pub struct CoverArtData {
    /// 图片数据（引用计数，clone 零拷贝）
    pub data: Bytes,
    /// MIME 类型
    pub mime_type: String,
    /// 缓存键（包含更新时间）
    pub cache_key: String,
    /// 更新时间戳（毫秒）
    pub last_modified: i64,
}

/// 封面读取结果（包含数据和 MIME 类型）
#[derive(Debug, Clone)]
pub struct CoverArtBytes {
    /// 图片数据（引用计数，clone 零拷贝）
    pub data: Bytes,
    /// MIME 类型
    pub mime_type: String,
}

/// 封面艺术读取器 trait
#[async_trait]
pub trait CoverArtReader: Send + Sync {
    /// 根据封面来源读取封面数据（包含 MIME 类型）
    async fn read(&self, source: &CoverSource) -> Result<CoverArtBytes, QueryError>;
}

/// 封面艺术缓存 trait
#[async_trait]
pub trait CoverArtCache: Send + Sync {
    /// 根据 cache_key 获取缓存的封面数据
    async fn get(&self, cache_key: &str) -> Option<CoverArtData>;
    
    /// 将封面数据存入缓存
    async fn put(&self, cache_key: &str, data: CoverArtData);
    
    /// 使缓存失效
    async fn invalidate(&self, cache_key: &str);
}

/// 封面来源信息（用于延迟加载）
#[derive(Debug, Clone)]
pub enum CoverSource {
    /// 嵌入封面（从音频文件中提取）
    Embedded { protocol: String, path: String },
    /// 外部封面文件
    External { protocol: String, path: String },
}

// ============================================================================
// 责任链模式：封面解析器
// ============================================================================

/// 封面解析上下文（传递给责任链的数据）
struct ResolveContext<'a> {
    dao: &'a (dyn CoverArtDao + Send + Sync),
    wildcards: &'a [String],
}

/// 封面解析器 trait（责任链节点）
#[async_trait]
trait CoverResolver: Send + Sync {
    /// 尝试解析封面，返回 Some 表示找到，None 表示传递给下一个
    async fn resolve(&self, ctx: &ResolveContext<'_>) -> Option<CoverSource>;
}

// ============================================================================
// AudioFile 解析器
// ============================================================================

/// 音频文件封面解析器（从 cover_art 表查询）
struct AudioFileCoverArtResolver {
    audio_file_id: i64,
}

#[async_trait]
impl CoverResolver for AudioFileCoverArtResolver {
    async fn resolve(&self, ctx: &ResolveContext<'_>) -> Option<CoverSource> {
        let cover = ctx.dao
            .get_cover_art_by_audio_file(self.audio_file_id)
            .await
            .ok()??;
        
        if cover.source == "embedded" {
            Some(CoverSource::Embedded { protocol: cover.protocol, path: cover.path })
        } else {
            Some(CoverSource::External { protocol: cover.protocol, path: cover.path })
        }
    }
}

// ============================================================================
// Album 解析器
// ============================================================================

/// 专辑直接关联封面解析器
struct AlbumDirectCoverResolver {
    album_id: i64,
}

#[async_trait]
impl CoverResolver for AlbumDirectCoverResolver {
    async fn resolve(&self, ctx: &ResolveContext<'_>) -> Option<CoverSource> {
        ctx.dao
            .get_cover_art_paths_by_album(self.album_id)
            .await
            .ok()?
            .first()
            .map(|c| CoverSource::External { protocol: c.protocol.clone(), path: c.path.clone() })
    }
}

/// 专辑目录通配符匹配解析器
struct AlbumPatternResolver {
    album_id: i64,
}

#[async_trait]
impl CoverResolver for AlbumPatternResolver {
    async fn resolve(&self, ctx: &ResolveContext<'_>) -> Option<CoverSource> {
        let locations = ctx.dao.get_album_locations(self.album_id).await.ok()?;
        let primary_loc = locations.first()?;
        
        let covers = ctx.dao
            .get_cover_art_paths_by_prefix(&primary_loc.protocol, &primary_loc.path)
            .await
            .ok()?;
        
        select_best_cover_breadth_first(&covers, &primary_loc.path, ctx.wildcards)
            .map(|c| CoverSource::External { protocol: c.protocol.clone(), path: c.path.clone() })
    }
}

/// 专辑音频文件封面解析器（从 cover_art 表查询，优先 external）
struct AlbumAudioFileCoverResolver {
    album_id: i64,
}

#[async_trait]
impl CoverResolver for AlbumAudioFileCoverResolver {
    async fn resolve(&self, ctx: &ResolveContext<'_>) -> Option<CoverSource> {
        let mut covers = ctx.dao.get_album_audio_file_covers(self.album_id).await.ok()?;
        if covers.is_empty() {
            return None;
        }
        
        // 按 external 优先排序（非 embedded 排前面）
        covers.sort_by(|a, b| {
            let a_is_embedded = a.source == "embedded";
            let b_is_embedded = b.source == "embedded";
            a_is_embedded.cmp(&b_is_embedded)
        });
        
        let cover = &covers[0];
        if cover.source == "embedded" {
            Some(CoverSource::Embedded { protocol: cover.protocol.clone(), path: cover.path.clone() })
        } else {
            Some(CoverSource::External { protocol: cover.protocol.clone(), path: cover.path.clone() })
        }
    }
}

// ============================================================================
// Artist 解析器
// ============================================================================

/// 艺术家直接关联封面解析器
struct ArtistDirectCoverResolver {
    artist_id: i64,
}

#[async_trait]
impl CoverResolver for ArtistDirectCoverResolver {
    async fn resolve(&self, ctx: &ResolveContext<'_>) -> Option<CoverSource> {
        ctx.dao
            .get_cover_art_by_artist(self.artist_id)
            .await
            .ok()
            .flatten()
            .map(|c| CoverSource::External { protocol: c.protocol, path: c.path })
    }
}

/// 艺术家专辑封面解析器（复用专辑解析链）
struct ArtistAlbumCoverResolver {
    artist_id: i64,
}

#[async_trait]
impl CoverResolver for ArtistAlbumCoverResolver {
    async fn resolve(&self, ctx: &ResolveContext<'_>) -> Option<CoverSource> {
        let album_id = ctx.dao.get_first_album_id_by_artist(self.artist_id).await.ok()??;
        
        // 构建专辑解析链
        let resolvers: Vec<Box<dyn CoverResolver>> = vec![
            Box::new(AlbumDirectCoverResolver { album_id }),
            Box::new(AlbumPatternResolver { album_id }),
            Box::new(AlbumAudioFileCoverResolver { album_id }),
        ];
        
        run_resolver_chain(&resolvers, ctx).await
    }
}

/// 艺术家音频文件封面解析器（从 cover_art 表查询，优先 external）
struct ArtistAudioFileCoverResolver {
    artist_id: i64,
}

#[async_trait]
impl CoverResolver for ArtistAudioFileCoverResolver {
    async fn resolve(&self, ctx: &ResolveContext<'_>) -> Option<CoverSource> {
        let mut covers = ctx.dao.get_artist_audio_file_covers(self.artist_id).await.ok()?;
        if covers.is_empty() {
            return None;
        }
        
        // 按 external 优先排序（非 embedded 排前面）
        covers.sort_by(|a, b| {
            let a_is_embedded = a.source == "embedded";
            let b_is_embedded = b.source == "embedded";
            a_is_embedded.cmp(&b_is_embedded)
        });
        
        let cover = &covers[0];
        if cover.source == "embedded" {
            Some(CoverSource::Embedded { protocol: cover.protocol.clone(), path: cover.path.clone() })
        } else {
            Some(CoverSource::External { protocol: cover.protocol.clone(), path: cover.path.clone() })
        }
    }
}

// ============================================================================
// Playlist 解析器
// ============================================================================

/// 播放列表嵌入封面解析器
struct PlaylistEmbeddedResolver {
    path: String,
    has_embedded: bool,
}

#[async_trait]
impl CoverResolver for PlaylistEmbeddedResolver {
    async fn resolve(&self, _ctx: &ResolveContext<'_>) -> Option<CoverSource> {
        if self.has_embedded {
            let (protocol, path) = parse_media_path(&self.path);
            Some(CoverSource::Embedded { protocol, path })
        } else {
            None
        }
    }
}

/// 播放列表目录封面解析器
struct PlaylistPatternResolver {
    path: String,
}

#[async_trait]
impl CoverResolver for PlaylistPatternResolver {
    async fn resolve(&self, ctx: &ResolveContext<'_>) -> Option<CoverSource> {
        let (protocol, file_path) = parse_media_path(&self.path);
        let dir_path = extract_parent_path(&file_path);
        
        let covers = ctx.dao
            .get_cover_art_paths_by_prefix(&protocol, &dir_path)
            .await
            .ok()?;
        
        select_best_cover_breadth_first(&covers, &dir_path, ctx.wildcards)
            .map(|c| CoverSource::External { protocol: c.protocol.clone(), path: c.path.clone() })
    }
}

// ============================================================================
// 责任链执行器
// ============================================================================

/// 执行解析器责任链
async fn run_resolver_chain(
    resolvers: &[Box<dyn CoverResolver>],
    ctx: &ResolveContext<'_>,
) -> Option<CoverSource> {
    for resolver in resolvers {
        if let Some(source) = resolver.resolve(ctx).await {
            return Some(source);
        }
    }
    None
}

// ============================================================================
// GetCoverArt 主服务
// ============================================================================

pub struct GetCoverArt {
    cover_art_dao: Arc<dyn CoverArtDao + Send + Sync>,
    cover_art_config: Arc<dyn CoverArtConfig + Send + Sync>,
    cover_art_reader: Arc<dyn CoverArtReader + Send + Sync>,
    cover_art_cache: Arc<dyn CoverArtCache + Send + Sync>,
}

impl GetCoverArt {
    pub fn new(
        cover_art_dao: Arc<dyn CoverArtDao + Send + Sync>,
        cover_art_config: Arc<dyn CoverArtConfig + Send + Sync>,
        cover_art_reader: Arc<dyn CoverArtReader + Send + Sync>,
        cover_art_cache: Arc<dyn CoverArtCache + Send + Sync>,
    ) -> Self {
        Self {
            cover_art_dao,
            cover_art_config,
            cover_art_reader,
            cover_art_cache,
        }
    }

    /// 根据 artwork_id 获取封面数据，如果找不到返回占位图
    /// 优化流程：
    /// 1. 查询 ID 和 last_modified 构建 cache_key
    /// 2. 检查带 size 的缓存 -> 命中直接返回
    /// 3. 检查原图缓存 -> 命中则缩放后返回（并缓存缩放结果）
    /// 4. 从磁盘读取 -> 同时缓存原图和缩放后的图片
    pub async fn get_or_placeholder(&self, artwork_id_str: &str, size: Option<u32>) -> Result<CoverArtData, QueryError> {
        let artwork_id = ArtworkId::parse(artwork_id_str)
            .map_err(|e| QueryError::InvalidInput(e.to_string()))?;

        // 1. 查询对象的 ID 和 last_modified 构建 cache_key
        let (base_cache_key, last_modified) = self.get_cache_key(&artwork_id).await?;
        
        // 构建带 size 的 cache_key
        let needs_resize = matches!(size, Some(s) if s > 0);
        let sized_cache_key = if needs_resize {
            format!("{}-{}", base_cache_key, size.unwrap())
        } else {
            base_cache_key.clone()
        };

        // 2. 先查带 size 的缓存
        if let Some(cached) = self.cover_art_cache.get(&sized_cache_key).await {
            log::debug!("Cache hit for sized cover art: {}", sized_cache_key);
            return Ok(cached);
        }
        
        log::debug!("Cache miss for sized cover art: {}", sized_cache_key);

        // 3. 查原图缓存（仅当需要缩放时才查，否则上面已经查过了）
        if needs_resize {
            if let Some(cached) = self.cover_art_cache.get(&base_cache_key).await {
                log::debug!("Cache hit for original cover art: {}", base_cache_key);
                // 缩放并缓存
                return self.resize_and_cache(cached, size.unwrap(), &sized_cache_key, last_modified).await;
            }
            log::debug!("Cache miss for original cover art: {}", base_cache_key);
        }

        // 4. 缓存未命中，执行解析链获取封面来源
        let source = self.resolve_cover_source(&artwork_id).await?;
        
        // 5. 从磁盘读取原图
        let original_data = match &source {
            Some(src) => {
                match self.cover_art_reader.read(src).await {
                    Ok(result) => Some((result.data, result.mime_type)),
                    Err(e) => {
                        log::warn!("Failed to read cover: {}", e);
                        None
                    }
                }
            }
            None => None,
        };

        // 6. 如果有原图数据，缓存原图和缩放后的图片
        if let Some((data, mime_type)) = original_data {
            let original = CoverArtData {
                data: data.clone(),
                mime_type: mime_type.clone(),
                cache_key: base_cache_key.clone(),
                last_modified,
            };
            
            // 缓存原图
            self.cover_art_cache.put(&base_cache_key, original.clone()).await;
            log::debug!("Cached original cover art: {}", base_cache_key);
            
            // 如果需要缩放，缩放并缓存
            if needs_resize {
                return self.resize_and_cache(original, size.unwrap(), &sized_cache_key, last_modified).await;
            }
            
            Ok(original)
        } else {
            // 7. 返回占位图（支持缩放）
            Ok(self.get_placeholder(&artwork_id.kind, size))
        }
    }
    
    /// 获取 cache_key（仅查询 ID 和 last_modified）
    async fn get_cache_key(&self, artwork_id: &ArtworkId) -> Result<(String, i64), QueryError> {
        match artwork_id.kind {
            ArtworkKind::AudioFile => {
                let info = self.cover_art_dao
                    .get_audio_file_cover_info(artwork_id.id)
                    .await?
                    .ok_or_else(|| QueryError::NotFound(format!("Audio file {} not found", artwork_id.id)))?;
                let last_modified = info.updated_at.and_utc().timestamp_millis();
                Ok((format!("af-{}-{}", artwork_id.id, last_modified), last_modified))
            }
            ArtworkKind::Album => {
                let info = self.cover_art_dao
                    .get_album_cover_info(artwork_id.id)
                    .await?
                    .ok_or_else(|| QueryError::NotFound(format!("Album {} not found", artwork_id.id)))?;
                let last_modified = info.updated_at.and_utc().timestamp_millis();
                Ok((format!("al-{}-{}", artwork_id.id, last_modified), last_modified))
            }
            ArtworkKind::Artist => {
                let info = self.cover_art_dao
                    .get_artist_cover_info(artwork_id.id)
                    .await?
                    .ok_or_else(|| QueryError::NotFound(format!("Artist {} not found", artwork_id.id)))?;
                let last_modified = info.updated_at.and_utc().timestamp_millis();
                Ok((format!("ar-{}-{}", artwork_id.id, last_modified), last_modified))
            }
            ArtworkKind::Playlist => {
                match self.cover_art_dao.get_playlist_cover_info(artwork_id.id).await? {
                    Some(info) => {
                        let last_modified = info.updated_at.and_utc().timestamp_millis();
                        Ok((format!("pl-{}-{}", artwork_id.id, last_modified), last_modified))
                    }
                    None => Ok((format!("pl-{}-0", artwork_id.id), 0)),
                }
            }
        }
    }
    
    /// 缩放图片并缓存结果
    async fn resize_and_cache(
        &self,
        original: CoverArtData,
        size: u32,
        sized_cache_key: &str,
        last_modified: i64,
    ) -> Result<CoverArtData, QueryError> {
        match resize_image(&original.data, size, &original.mime_type) {
            Ok((resized_data, mime_type)) => {
                let resized = CoverArtData {
                    data: resized_data,
                    mime_type,
                    cache_key: sized_cache_key.to_string(),
                    last_modified,
                };
                // 缓存缩放后的图片
                self.cover_art_cache.put(sized_cache_key, resized.clone()).await;
                log::debug!("Cached resized cover art: {}", sized_cache_key);
                Ok(resized)
            }
            Err(e) => {
                log::warn!("Failed to resize image: {}, returning original", e);
                // 缩放失败，返回原图
                Ok(original)
            }
        }
    }

    /// 获取占位图（支持缩放）
    /// - Artist 使用艺术家占位图
    /// - 其他使用专辑占位图
    fn get_placeholder(&self, kind: &ArtworkKind, size: Option<u32>) -> CoverArtData {
        let (placeholder_path, base_cache_key) = match kind {
            ArtworkKind::Artist => (
                self.cover_art_config.artist_placeholder_path(),
                "placeholder-artist",
            ),
            _ => (
                self.cover_art_config.album_placeholder_path(),
                "placeholder-album",
            ),
        };
        
        // 构建带 size 的 cache_key
        let cache_key = match size {
            Some(s) if s > 0 => format!("{}-{}", base_cache_key, s),
            _ => base_cache_key.to_string(),
        };
        
        // 尝试从文件读取占位图
        if let Some(path) = placeholder_path {
            if let Ok(data) = std::fs::read(&path) {
                let mime_type = if path.ends_with(".webp") {
                    "image/webp"
                } else if path.ends_with(".png") {
                    "image/png"
                } else {
                    "image/jpeg"
                };
                
                let data = Bytes::from(data);
                
                // 如果需要缩放
                if let Some(s) = size {
                    if s > 0 {
                        match resize_image(&data, s, mime_type) {
                            Ok((resized_data, resized_mime)) => {
                                return CoverArtData {
                                    data: resized_data,
                                    mime_type: resized_mime,
                                    cache_key,
                                    last_modified: 0,
                                };
                            }
                            Err(e) => {
                                log::warn!("Failed to resize placeholder: {}", e);
                            }
                        }
                    }
                }
                
                // 返回原图
                return CoverArtData {
                    data,
                    mime_type: mime_type.to_string(),
                    cache_key,
                    last_modified: 0,
                };
            }
        }
        
        // 回退到内嵌的透明 PNG
        log::warn!("Placeholder file not found, using fallback PNG");
        CoverArtData {
            data: Bytes::from_static(FALLBACK_PNG),
            mime_type: "image/png".to_string(),
            cache_key: "placeholder-fallback".to_string(),
            last_modified: 0,
        }
    }

    /// 根据 artwork_id 解析封面来源（仅返回 CoverSource，不包含 cache_key）
    async fn resolve_cover_source(&self, artwork_id: &ArtworkId) -> Result<Option<CoverSource>, QueryError> {
        let wildcards = self.cover_art_config.cover_art_wildcards();
        let ctx = ResolveContext {
            dao: self.cover_art_dao.as_ref(),
            wildcards: &wildcards,
        };

        match artwork_id.kind {
            ArtworkKind::AudioFile => {
                let resolvers: Vec<Box<dyn CoverResolver>> = vec![
                    Box::new(AudioFileCoverArtResolver { audio_file_id: artwork_id.id }),
                ];
                Ok(run_resolver_chain(&resolvers, &ctx).await)
            }
            ArtworkKind::Album => {
                let resolvers: Vec<Box<dyn CoverResolver>> = vec![
                    Box::new(AlbumDirectCoverResolver { album_id: artwork_id.id }),
                    Box::new(AlbumPatternResolver { album_id: artwork_id.id }),
                    Box::new(AlbumAudioFileCoverResolver { album_id: artwork_id.id }),
                ];
                Ok(run_resolver_chain(&resolvers, &ctx).await)
            }
            ArtworkKind::Artist => {
                let resolvers: Vec<Box<dyn CoverResolver>> = vec![
                    Box::new(ArtistDirectCoverResolver { artist_id: artwork_id.id }),
                    Box::new(ArtistAlbumCoverResolver { artist_id: artwork_id.id }),
                    Box::new(ArtistAudioFileCoverResolver { artist_id: artwork_id.id }),
                ];
                Ok(run_resolver_chain(&resolvers, &ctx).await)
            }
            ArtworkKind::Playlist => {
                // Playlist 需要额外信息来构建解析器
                let info = self.cover_art_dao.get_playlist_cover_info(artwork_id.id).await?;
                match info {
                    Some(info) => {
                        let resolvers: Vec<Box<dyn CoverResolver>> = vec![
                            Box::new(PlaylistEmbeddedResolver {
                                path: info.path.clone(),
                                has_embedded: info.has_embedded,
                            }),
                            Box::new(PlaylistPatternResolver { path: info.path }),
                        ];
                        Ok(run_resolver_chain(&resolvers, &ctx).await)
                    }
                    None => Ok(None),
                }
            }
        }
    }
}

// ============================================================================
// 工具函数
// ============================================================================

/// 从路径中提取父目录
fn extract_parent_path(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 解析媒体路径（protocol://path）
fn parse_media_path(media_path: &str) -> (String, String) {
    if let Some(idx) = media_path.find("://") {
        let protocol = media_path[..idx].to_string();
        let path = media_path[idx + 3..].to_string();
        (protocol, path)
    } else {
        ("file".to_string(), media_path.to_string())
    }
}

/// 计算路径深度（斜杠数量）
fn path_depth(path: &str) -> usize {
    path.chars().filter(|&c| c == '/').count()
}

/// 从路径中提取文件名
fn extract_filename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// 检查文件名是否匹配通配符模式（不区分大小写）
fn matches_wildcard(filename: &str, pattern: &str) -> bool {
    let filename_lower = filename.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    
    if pattern_lower == "*" {
        return true;
    }
    
    // 处理 "name.*" 格式的通配符
    if let Some(prefix) = pattern_lower.strip_suffix(".*") {
        if let Some(dot_pos) = filename_lower.rfind('.') {
            let name_without_ext = &filename_lower[..dot_pos];
            return name_without_ext == prefix;
        }
        return filename_lower == prefix;
    }
    
    // 处理 "*.ext" 格式的通配符
    if let Some(suffix) = pattern_lower.strip_prefix("*.") {
        return filename_lower.ends_with(&format!(".{}", suffix));
    }
    
    // 精确匹配
    filename_lower == pattern_lower
}

/// 广度优先选择最佳封面（按深度和路径排序，每个目录按通配符顺序过滤）
fn select_best_cover_breadth_first<'a>(
    covers: &'a [CoverArtPath],
    _base_path: &str,
    wildcards: &[String],
) -> Option<&'a CoverArtPath> {
    if covers.is_empty() {
        return None;
    }

    // 按深度排序，同深度按完整路径排序
    let mut sorted: Vec<&CoverArtPath> = covers.iter().collect();
    sorted.sort_by(|a, b| {
        let depth_a = path_depth(&a.path);
        let depth_b = path_depth(&b.path);
        if depth_a != depth_b {
            return depth_a.cmp(&depth_b);
        }
        a.path.cmp(&b.path)
    });

    // 按深度和目录分组，每组按通配符顺序过滤
    let mut current_depth = path_depth(&sorted[0].path);
    let mut current_dir = extract_dir(&sorted[0].path);
    let mut current_group: Vec<&CoverArtPath> = Vec::new();

    for cover in sorted {
        let depth = path_depth(&cover.path);
        let dir = extract_dir(&cover.path);
        
        if depth != current_depth || dir != current_dir {
            if let Some(found) = filter_by_wildcards(&current_group, wildcards) {
                return Some(found);
            }
            current_depth = depth;
            current_dir = dir;
            current_group.clear();
        }
        current_group.push(cover);
    }

    filter_by_wildcards(&current_group, wildcards)
}

/// 提取路径的目录部分
fn extract_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(pos) => &path[..pos],
        None => "",
    }
}

/// 按通配符顺序过滤，返回第一个匹配的文件
fn filter_by_wildcards<'a>(
    covers: &[&'a CoverArtPath],
    wildcards: &[String],
) -> Option<&'a CoverArtPath> {
    for pattern in wildcards {
        for cover in covers {
            let filename = extract_filename(&cover.path);
            if matches_wildcard(filename, pattern) {
                return Some(*cover);
            }
        }
    }
    None
}

/// 获取文件名在通配符列表中的优先级（返回索引，越小优先级越高）
fn get_wildcard_priority(filename: &str, wildcards: &[String]) -> usize {
    for (i, pattern) in wildcards.iter().enumerate() {
        if matches_wildcard(filename, pattern) {
            return i;
        }
    }
    usize::MAX
}

/// 缩放图片到指定尺寸（保持宽高比）
/// 返回缩放后的图片数据和 MIME 类型
fn resize_image(data: &Bytes, size: u32, original_mime: &str) -> Result<(Bytes, String), String> {
    // 加载图片
    let img = image::load_from_memory(data)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    let (width, height) = (img.width(), img.height());
    
    // 如果原图已经小于目标尺寸，直接返回原图
    if width <= size && height <= size {
        return Ok((data.clone(), original_mime.to_string()));
    }
    
    // 计算缩放后的尺寸（保持宽高比）
    let (new_width, new_height) = if width > height {
        let ratio = size as f64 / width as f64;
        (size, (height as f64 * ratio) as u32)
    } else {
        let ratio = size as f64 / height as f64;
        ((width as f64 * ratio) as u32, size)
    };
    
    // 使用 Lanczos3 算法缩放（高质量）
    let resized = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
    
    // 编码为 JPEG（通用且压缩率好）
    let mut buffer = Cursor::new(Vec::new());
    resized.write_to(&mut buffer, ImageFormat::Jpeg)
        .map_err(|e| format!("Failed to encode image: {}", e))?;
    
    Ok((Bytes::from(buffer.into_inner()), "image/jpeg".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_media_path() {
        let (protocol, path) = parse_media_path("file:///music/album/song.mp3");
        assert_eq!(protocol, "file");
        assert_eq!(path, "/music/album/song.mp3");
    }

    #[test]
    fn test_extract_parent_path() {
        let parent = extract_parent_path("/music/album/song.mp3");
        assert_eq!(parent, "/music/album");
    }

    #[test]
    fn test_path_depth() {
        assert_eq!(path_depth("/music/artist/album/cover.jpg"), 4);
        assert_eq!(path_depth("/music/cover.jpg"), 2);
        assert_eq!(path_depth("cover.jpg"), 0);
    }

    #[test]
    fn test_matches_wildcard() {
        // cover.* 匹配
        assert!(matches_wildcard("cover.jpg", "cover.*"));
        assert!(matches_wildcard("Cover.PNG", "cover.*"));
        assert!(matches_wildcard("COVER.webp", "cover.*"));
        assert!(!matches_wildcard("front.jpg", "cover.*"));
        
        // folder.* 匹配
        assert!(matches_wildcard("folder.jpg", "folder.*"));
        assert!(matches_wildcard("Folder.png", "folder.*"));
        
        // *.jpg 匹配
        assert!(matches_wildcard("anything.jpg", "*.jpg"));
        assert!(matches_wildcard("cover.JPG", "*.jpg"));
        
        // * 匹配所有
        assert!(matches_wildcard("anything.jpg", "*"));
        assert!(matches_wildcard("cover.png", "*"));
    }

    #[test]
    fn test_get_wildcard_priority() {
        let wildcards = vec![
            "cover.*".to_string(),
            "folder.*".to_string(),
            "front.*".to_string(),
            "*".to_string(),
        ];
        
        assert_eq!(get_wildcard_priority("cover.jpg", &wildcards), 0);
        assert_eq!(get_wildcard_priority("folder.png", &wildcards), 1);
        assert_eq!(get_wildcard_priority("front.webp", &wildcards), 2);
        assert_eq!(get_wildcard_priority("random.jpg", &wildcards), 3);
    }

    #[test]
    fn test_select_best_cover_breadth_first() {
        let wildcards = vec![
            "cover.*".to_string(),
            "folder.*".to_string(),
            "*".to_string(),
        ];
        
        let covers = vec![
            CoverArtPath { protocol: "file".to_string(), path: "/music/artist/album/cover.jpg".to_string() },
            CoverArtPath { protocol: "file".to_string(), path: "/music/artist/cover.jpg".to_string() },
            CoverArtPath { protocol: "file".to_string(), path: "/music/cover.jpg".to_string() },
        ];
        
        let best = select_best_cover_breadth_first(&covers, "/music/artist", &wildcards);
        assert!(best.is_some());
        assert_eq!(best.unwrap().path, "/music/cover.jpg");
    }

    #[test]
    fn test_select_best_cover_with_wildcard_priority() {
        let wildcards = vec![
            "cover.*".to_string(),
            "folder.*".to_string(),
            "*".to_string(),
        ];
        
        let covers = vec![
            CoverArtPath { protocol: "file".to_string(), path: "/music/album/folder.jpg".to_string() },
            CoverArtPath { protocol: "file".to_string(), path: "/music/album/cover.png".to_string() },
            CoverArtPath { protocol: "file".to_string(), path: "/music/album/art.jpg".to_string() },
        ];
        
        let best = select_best_cover_breadth_first(&covers, "/music/album", &wildcards);
        assert!(best.is_some());
        assert_eq!(best.unwrap().path, "/music/album/cover.png");
    }
}
