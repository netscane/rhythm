use crate::{
    event::DomainEvent,
    value::{AlbumId, AudioFileId, CoverArtId, MediaPath},
};
use chrono::NaiveDateTime;
use std::fmt;
use thiserror::Error;

// 封面来源类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoverSourceType {
    Embedded,   // 嵌入在音频文件中
    External,   // 外部文件
    Downloaded, // 从网络下载
    Generated,  // 自动生成
    Manual,     // 人工上传
}

impl fmt::Display for CoverSourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoverSourceType::Embedded => write!(f, "embedded"),
            CoverSourceType::External => write!(f, "external"),
            CoverSourceType::Downloaded => write!(f, "downloaded"),
            CoverSourceType::Generated => write!(f, "generated"),
            CoverSourceType::Manual => write!(f, "manual"),
        }
    }
}

impl From<&str> for CoverSourceType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "embedded" => CoverSourceType::Embedded,
            "external" => CoverSourceType::External,
            "downloaded" => CoverSourceType::Downloaded,
            "generated" => CoverSourceType::Generated,
            "manual" => CoverSourceType::Manual,
            _ => CoverSourceType::External, // default
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverArtMeta {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub format: Option<CoverFormat>,
    pub size: i64,
    pub path: MediaPath,
    pub mtime: NaiveDateTime,
    pub atime: NaiveDateTime,
    pub ctime: NaiveDateTime,
}

// 封面格式
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoverFormat {
    Jpeg,
    Png,
    WebP,
    Gif,
    Bmp,
    Tiff,
}

impl fmt::Display for CoverFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoverFormat::Jpeg => write!(f, "jpeg"),
            CoverFormat::Png => write!(f, "png"),
            CoverFormat::WebP => write!(f, "webp"),
            CoverFormat::Gif => write!(f, "gif"),
            CoverFormat::Bmp => write!(f, "bmp"),
            CoverFormat::Tiff => write!(f, "tiff"),
        }
    }
}

impl From<&str> for CoverFormat {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "jpg" | "jpeg" => CoverFormat::Jpeg,
            "png" => CoverFormat::Png,
            "webp" => CoverFormat::WebP,
            "gif" => CoverFormat::Gif,
            "bmp" => CoverFormat::Bmp,
            "tiff" | "tif" => CoverFormat::Tiff,
            _ => CoverFormat::Jpeg, // 默认格式
        }
    }
}

// 封面聚合根
#[derive(Debug, Clone)]
pub struct CoverArt {
    pub id: CoverArtId,
    pub audio_file_id: Option<AudioFileId>,
    pub path: MediaPath, // 文件路径
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub format: Option<CoverFormat>, // 图片格式
    pub file_size: i64,              // 文件大小（字节）
    pub source: CoverSourceType,     // 原始来源信息
    pub version: i64,                // 版本号（乐观锁）
    pending_events: Vec<CoverArtEvent>,
}

pub struct CoverArtDTO {
    pub audio_file_id: Option<AudioFileId>,
    pub album_id: Option<AlbumId>,
    pub path: MediaPath,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub format: Option<CoverFormat>,
    pub file_size: i64,
    pub source: CoverSourceType,
}

// 领域事件
#[derive(Debug, Clone)]
pub struct CoverArtCreated {
    pub cover_art_id: CoverArtId,
    pub path: MediaPath,
    pub source: CoverSourceType,
    pub audio_file_id: Option<AudioFileId>,
    pub album_id: Option<AlbumId>,
}

#[derive(Debug, Clone)]
pub struct CoverArtDeleted {
    pub cover_art_id: CoverArtId,
    pub path: MediaPath,
    pub audio_file_id: Option<AudioFileId>,
}

#[derive(Debug, Clone)]
pub struct NoCoverArt {
    pub path: MediaPath,
}

#[derive(Debug, Clone)]
pub struct CoverArtEvent {
    pub cover_art_id: CoverArtId,
    pub version: i64,
    pub kind: CoverArtEventKind,
}

impl DomainEvent for CoverArtEvent {
    fn aggregate_id(&self) -> i64 {
        self.cover_art_id.as_i64()
    }
    fn version(&self) -> i64 {
        self.version
    }
}
#[derive(Debug, Clone)]
pub enum CoverArtEventKind {
    Created(CoverArtCreated),
    Deleted(CoverArtDeleted),
    NoCoverArt(NoCoverArt),
}

// 错误类型
#[derive(Error, Debug)]
pub enum CoverArtError {
    #[error("Invalid dimensions: width={width}, height={height}")]
    InvalidDimensions { width: i32, height: i32 },
    #[error("Invalid file size: {size} bytes")]
    InvalidFileSize { size: i64 },
    #[error("Unsupported format: {format}")]
    UnsupportedFormat { format: String },
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    #[error("Version conflict: expected {expected}, got {actual}")]
    VersionConflict { expected: i64, actual: i64 },
    #[error("Cover art already exists for audio file {audio_file_id}")]
    CoverArtAlreadyExists { audio_file_id: i64 },
    #[error("Database error: {message}")]
    DatabaseError { message: String },
    #[error("File system error: {message}")]
    FileSystemError { message: String },
}

// 业务规则验证
impl CoverArt {
    // 创建新的封面艺术
    pub fn from_dto(id: CoverArtId, dto: CoverArtDTO) -> Result<Self, CoverArtError> {
        // 验证业务规则
        Self::validate_file_size(dto.file_size, &dto.source)?;

        let created_evt = CoverArtEvent {
            cover_art_id: id.clone(),
            version: 0,
            kind: CoverArtEventKind::Created(CoverArtCreated {
                cover_art_id: id.clone(),
                path: dto.path.clone(),
                source: dto.source.clone(),
                audio_file_id: dto.audio_file_id.clone(),
                album_id: dto.album_id.clone(),
            }),
        };

        Ok(Self {
            id,
            audio_file_id: dto.audio_file_id,
            path: dto.path,
            width: dto.width,
            height: dto.height,
            format: dto.format,
            file_size: dto.file_size,
            source: dto.source,
            version: 0,
            pending_events: vec![created_evt],
        })
    }

    // 验证文件大小
    fn validate_file_size(file_size: i64, _source: &CoverSourceType) -> Result<(), CoverArtError> {
        if file_size == 0 {
            return Err(CoverArtError::InvalidFileSize { size: file_size });
        }
        // 500MB 限制，支持超高分辨率图片
        if file_size > 500 * 1024 * 1024 {
            return Err(CoverArtError::InvalidFileSize { size: file_size });
        }
        Ok(())
    }

    // 检查是否为高质量封面
    pub fn is_high_quality(&self) -> bool {
        let max_dimension = self.width.unwrap_or(0).max(self.height.unwrap_or(0));
        max_dimension >= 800
    }

    // 获取宽高比
    pub fn aspect_ratio(&self) -> f32 {
        let height = self.height.unwrap_or(0);
        let width = self.width.unwrap_or(0);
        if height == 0 {
            return 1.0;
        }
        width as f32 / height as f32
    }

    // 检查是否为正方形封面
    pub fn is_square(&self) -> bool {
        let width = self.width.unwrap_or(0) as f32;
        let height = self.height.unwrap_or(0) as f32;
        (width - height).abs() < 1.0
    }

    // 获取待处理的事件
    pub fn take_events(&mut self) -> Vec<CoverArtEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn bind_to_audio_file(&mut self, audio_file_id: AudioFileId) {
        self.audio_file_id = Some(audio_file_id);
    }
}

// ============== 仓储接口 ==============

use async_trait::async_trait;

/// CoverArtRepository 封面艺术仓储接口
#[async_trait]
pub trait CoverArtRepository: Send + Sync {
    /// save 保存封面艺术
    async fn save(&self, mut cover_art: CoverArt) -> Result<CoverArt, CoverArtError>;

    /// find_by_id 根据ID加载封面艺术
    async fn find_by_id(&self, id: &CoverArtId) -> Result<Option<CoverArt>, CoverArtError>;

    /// find_by_album_id 根据专辑ID查找封面艺术
    async fn find_by_album_id(&self, album_id: &AlbumId) -> Result<Vec<CoverArt>, CoverArtError>;

    /// delete 删除封面艺术
    async fn delete(&self, id: &CoverArtId) -> Result<(), CoverArtError>;
}
