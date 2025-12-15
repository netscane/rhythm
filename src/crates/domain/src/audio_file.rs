use crate::event::DomainEvent;
use crate::value::{
    AlbumId, ArtistId, AudioFileId, AudioMetadata, AudioQuality, FileMeta, GenreId, LibraryId,
    MediaPath, Participant,
};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use thiserror::Error;
/// AudioFileMeta 文件内标签解析出的原始元数据（未清洗的脏数据）
#[derive(Debug, Clone)]
pub struct AudioFileMeta {
    // 基础标签
    pub title: String,
    // 曲目信息
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,

    // 发行相关
    pub year: Option<i32>,          // 普通标签里的年份
    pub date: Option<i32>,          // 具体日期（若有）
    pub original_year: Option<i32>, // 原始发行年份
    pub original_date: Option<i32>, // 原始发行日期
    pub release_year: Option<i32>,  // 再版年份
    pub release_date: Option<i32>,  // 再版日期
    pub compilation: bool,          // 是否为合辑

    // 节奏信息
    pub bpm: Option<i32>, // 每分钟节拍数
}

impl From<AudioMetadata> for AudioFileMeta {
    fn from(meta: AudioMetadata) -> Self {
        Self {
            title: meta.title,
            track_number: meta.track_number,
            disc_number: None,
            year: meta.year,
            date: None,
            original_year: None,
            original_date: None,
            release_year: None,
            release_date: None,
            compilation: false,
            bpm: None,
        }
    }
}

/// AudioFile AudioFile聚合根，代表一个音频媒体文件
#[derive(Debug, Clone)]
pub struct AudioFile {
    pub id: AudioFileId,

    pub library_id: LibraryId,

    // 文件存储属性
    pub path: MediaPath,      // 文件路径
    pub size: i64,            // 文件大小
    pub suffix: String,       // 扩展名（mp3, flac）
    pub hash: Option<String>, // 文件哈希（唯一性）

    // 技术属性
    pub duration: i64,    // 时长（秒）
    pub bit_rate: i32,    // 比特率
    pub bit_depth: i32,   // 位深度
    pub sample_rate: i32, // 采样率
    pub channels: i32,    // 声道数

    // 绑定后的引用
    pub album: Option<AlbumId>,
    pub artist: Option<ArtistId>,
    pub participants: Vec<Participant>,
    pub genre: Option<GenreId>,
    pub genres: Vec<GenreId>,

    // 原始标签元数据
    pub meta: AudioFileMeta,

    // 是否包含封面
    pub has_cover_art: bool,

    // 时间戳
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    // 领域事件
    pub events: Vec<AudioFileEvent>,

    // 版本号
    pub version: i64,
}

impl From<&mut AudioFile> for FileMeta {
    fn from(audio_file: &mut AudioFile) -> Self {
        Self {
            path: audio_file.path.clone(),
            dir_path: audio_file.path.clone(),
            size: audio_file.size,
            suffix: audio_file.suffix.clone(),
            mtime: audio_file.created_at,
            atime: audio_file.created_at,
            ctime: audio_file.created_at,
            hash: audio_file.hash.clone(),
        }
    }
}

impl AudioFile {
    /// new 创建新的AudioFile
    pub fn new(
        id: AudioFileId,
        library_id: LibraryId,
        path: MediaPath,
        size: i64,
        suffix: String,
        hash: Option<String>,
        duration: i64,
        bit_rate: i32,
        bit_depth: i32,
        sample_rate: i32,
        channels: i32,
        has_cover_art: bool,
        meta: AudioFileMeta,
    ) -> Self {
        let now = Utc::now().naive_utc();
        let audio_file = Self {
            id,
            library_id,
            path,
            size,
            suffix,
            hash,
            duration,
            bit_rate,
            bit_depth,
            sample_rate,
            channels,
            album: None,
            artist: None,
            genre: None,
            participants: Vec::new(),
            genres: Vec::new(),
            meta,
            has_cover_art,
            created_at: now,
            updated_at: now,
            events: Vec::new(),
            version: 0,
        };

        // 添加创建事件
        let mut audio_file = audio_file;
        audio_file.add_created_event();
        audio_file
    }

    /// bind_to_album 绑定到专辑
    pub fn bind_to_album(&mut self, album_id: AlbumId) -> Result<(), AudioFileError> {
        if self.album.is_some() {
            return Err(AudioFileError::AlreadyBoundToAlbum);
        }

        self.album = Some(album_id.clone());
        self.updated_at = Utc::now().naive_utc();
        self.events.push(AudioFileEvent {
            audio_file_id: self.id.clone(),
            version: self.version,
            kind: AudioFileEventKind::BoundToAlbum(AudioFileBoundToAlbum {
                album_id: album_id.clone(),
                audio_file_id: self.id.clone(),
                path: self.path.clone(),
                size: self.size,
                duration: self.duration,
                year: self.meta.year,
                track_number: self.meta.track_number,
                disc_number: self.meta.disc_number,
            }),
        });
        Ok(())
    }

    /// unbind_from_album 从专辑解绑
    pub fn unbind_from_album(&mut self) -> Result<(), AudioFileError> {
        if self.album.is_none() {
            return Err(AudioFileError::NotBoundToAlbum);
        }

        let old_album_id = self.album.take().unwrap();
        self.updated_at = Utc::now().naive_utc();
        self.events.push(AudioFileEvent {
            audio_file_id: self.id.clone(),
            version: self.version,
            kind: AudioFileEventKind::UnboundFromAlbum(AudioFileUnboundFromAlbum {
                album_id: old_album_id,
                audio_file_id: self.id.clone(),
                path: self.path.clone(),
                size: self.size,
                duration: self.duration,
                track_number: self.meta.track_number,
                disc_number: self.meta.disc_number,
            }),
        });
        Ok(())
    }

    pub fn add_participant(&mut self, participant: Participant) -> Result<(), AudioFileError> {
        if self.artist.is_none() {
            self.artist = Some(participant.artist_id.clone());
        }
        if !self.participants.iter().any(|p| p == &participant) {
            self.participants.push(participant.clone());
            self.updated_at = Utc::now().naive_utc();
            self.events.push(AudioFileEvent {
                audio_file_id: self.id.clone(),
                version: self.version,
                kind: AudioFileEventKind::ParticipantAdded(AudioFileParticipantAdded {
                    participant: participant.clone(),
                    all_participants: self.participants.clone(),
                    audio_file_id: self.id.clone(),
                    path: self.path.clone(),
                    size: self.size,
                    duration: self.duration,
                }),
            });
        }
        Ok(())
    }

    pub fn remove_participant(&mut self, participant: Participant) -> Result<(), AudioFileError> {
        if !self.participants.iter().any(|p| p == &participant) {
            return Err(AudioFileError::ParticipantNotFound(participant.clone()));
        }

        self.participants.retain(|p| p != &participant);
        self.updated_at = Utc::now().naive_utc();
        self.events.push(AudioFileEvent {
            audio_file_id: self.id.clone(),
            version: self.version,
            kind: AudioFileEventKind::ParticipantRemoved(AudioFileParticipantRemoved {
                participant: participant.clone(),
                all_participants: self.participants.clone(),
                audio_file_id: self.id.clone(),
                path: self.path.clone(),
                size: self.size,
                duration: self.duration,
            }),
        });
        Ok(())
    }

    pub fn bind_to_genre(&mut self, genre_id: GenreId) -> Result<(), AudioFileError> {
        if self.genre.is_none() {
            self.genre = Some(genre_id.clone());
        }
        if self.genres.contains(&genre_id) {
            return Ok(());
        }

        self.genres.push(genre_id.clone());
        self.updated_at = Utc::now().naive_utc();
        self.events.push(AudioFileEvent {
            audio_file_id: self.id.clone(),
            version: self.version,
            kind: AudioFileEventKind::GenreAdded(AudioFileGenreAdded {
                genre_id: genre_id.clone(),
                audio_file_id: self.id.clone(),
                size: self.size,
                duration: self.duration,
            }),
        });
        Ok(())
    }

    pub fn unbind_from_genre(&mut self, genre_id: GenreId) -> Result<(), AudioFileError> {
        if !self.genres.contains(&genre_id) {
            return Err(AudioFileError::NotBoundToGenre);
        }

        self.genres.retain(|g| g != &genre_id);
        self.updated_at = Utc::now().naive_utc();
        self.events.push(AudioFileEvent {
            audio_file_id: self.id.clone(),
            version: self.version,
            kind: AudioFileEventKind::UnboundFromGenre(AudioFileUnboundFromGenre {
                genre_id: genre_id.clone(),
                audio_file_id: self.id.clone(),
                size: self.size,
                duration: self.duration,
            }),
        });
        Ok(())
    }

    /// update_metadata 更新元数据
    pub fn update_metadata(&mut self, new_meta: AudioFileMeta) {
        self.meta = new_meta;
        self.updated_at = Utc::now().naive_utc();
        self.add_updated_event();
    }

    /// update_technical_info 更新技术信息
    pub fn update_technical_info(
        &mut self,
        duration: i64,
        bit_rate: i32,
        bit_depth: i32,
        sample_rate: i32,
        channels: i32,
    ) {
        self.duration = duration;
        self.bit_rate = bit_rate;
        self.bit_depth = bit_depth;
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.updated_at = Utc::now().naive_utc();
        self.add_updated_event();
    }

    /// quality 获取音频质量等级
    pub fn quality(&self) -> AudioQuality {
        // 根据格式、比特率、采样率、位深度计算质量等级
        let fmt = self.suffix.to_lowercase();
        let is_lossless_format = matches!(
            fmt.as_str(),
            "flac" | "alac" | "wav" | "aiff" | "ape" | "dsd"
        );

        // 定义阈值（经验值，可根据业务调整）
        let is_hires_sr = self.sample_rate >= 95_000 || self.bit_depth >= 24;
        let is_cd_quality = self.sample_rate >= 43_100 && self.bit_depth >= 16;

        if is_lossless_format {
            if is_hires_sr {
                return AudioQuality::HiRes;
            }
            if is_cd_quality {
                return AudioQuality::Lossless;
            }
            return AudioQuality::Standard;
        }

        // 非无损格式按码率/采样率粗略评估
        if self.sample_rate >= 47_000 && self.bit_rate >= 320 {
            return AudioQuality::Standard;
        }
        if self.sample_rate >= 43_100 && self.bit_rate >= 192 {
            return AudioQuality::Standard;
        }
        AudioQuality::Low
    }

    /// is_lossless 检查是否为无损格式
    pub fn is_lossless(&self) -> bool {
        matches!(
            self.suffix.to_lowercase().as_str(),
            "flac" | "alac" | "wav" | "aiff" | "ape" | "dsd"
        )
    }

    /// get_quality_score 获取质量评分
    pub fn get_quality_score(&self) -> f32 {
        let base_score = match self.quality() {
            AudioQuality::Lossless => 100.0,
            AudioQuality::HiRes => 90.0,
            AudioQuality::Standard => 80.0,
            AudioQuality::Low => 70.0,
        };

        // 根据技术参数调整分数
        let bit_rate_bonus = (self.bit_rate as f32 / 320.0).min(1.0) * 10.0;
        let sample_rate_bonus = (self.sample_rate as f32 / 96000.0).min(1.0) * 10.0;
        let bit_depth_bonus = (self.bit_depth as f32 / 24.0).min(1.0) * 10.0;

        base_score + bit_rate_bonus + sample_rate_bonus + bit_depth_bonus
    }

    /// delete 删除音频文件
    pub fn delete(&mut self) -> Result<(), AudioFileError> {
        if self.album.is_some() || !self.participants.is_empty() || !self.genres.is_empty() {
            return Err(AudioFileError::CannotDeleteBoundFile);
        }

        self.add_deleted_event();
        Ok(())
    }

    /// take_events 弹出所有未处理的领域事件
    pub fn take_events(&mut self) -> Vec<AudioFileEvent> {
        std::mem::take(&mut self.events)
    }

    // 私有方法：添加领域事件
    fn add_created_event(&mut self) {
        self.events.push(AudioFileEvent {
            audio_file_id: self.id.clone(),
            version: self.version,
            kind: AudioFileEventKind::Created(AudioFileCreated {
                library_id: self.library_id.clone(),
                audio_file_id: self.id.clone(),
                path: self.path.clone(),
                has_cover_art: self.has_cover_art,
            }),
        });
    }

    fn add_updated_event(&mut self) {
        self.events.push(AudioFileEvent {
            audio_file_id: self.id.clone(),
            version: self.version,
            kind: AudioFileEventKind::Updated(AudioFileUpdated {
                audio_file_id: self.id.clone(),
            }),
        });
    }

    fn add_deleted_event(&mut self) {
        self.events.push(AudioFileEvent {
            audio_file_id: self.id.clone(),
            version: self.version,
            kind: AudioFileEventKind::Deleted(AudioFileDeleted {}),
        });
    }
}

#[derive(Debug, Clone)]
pub struct AudioFileCreated {
    pub library_id: LibraryId,
    pub audio_file_id: AudioFileId,
    pub path: MediaPath,
    pub has_cover_art: bool,
}

#[derive(Debug, Clone)]
pub struct AudioFileBoundToAlbum {
    pub album_id: AlbumId,
    pub audio_file_id: AudioFileId,
    pub path: MediaPath,
    pub size: i64,
    pub duration: i64,
    pub year: Option<i32>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AudioFileParticipantAdded {
    pub participant: Participant,
    pub all_participants: Vec<Participant>,
    pub audio_file_id: AudioFileId,
    pub path: MediaPath,
    pub size: i64,
    pub duration: i64,
}

#[derive(Debug, Clone)]
pub struct AudioFileParticipantRemoved {
    pub participant: Participant,
    pub all_participants: Vec<Participant>,
    pub audio_file_id: AudioFileId,
    pub path: MediaPath,
    pub size: i64,
    pub duration: i64,
}
#[derive(Debug, Clone)]
pub struct AudioFileGenreAdded {
    pub genre_id: GenreId,
    pub audio_file_id: AudioFileId,
    pub size: i64,
    pub duration: i64,
}

#[derive(Debug, Clone)]
pub struct AudioFileGenreRemoved {
    pub genre_id: GenreId,
    pub audio_file_id: AudioFileId,
    pub size: i64,
    pub duration: i64,
}
#[derive(Debug, Clone)]
pub struct AudioFileUpdated {
    pub audio_file_id: AudioFileId,
}
#[derive(Debug, Clone)]
pub struct AudioFileUnboundFromAlbum {
    pub album_id: AlbumId,
    pub audio_file_id: AudioFileId,
    pub path: MediaPath,
    pub size: i64,
    pub duration: i64,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AudioFileUnboundFromGenre {
    pub genre_id: GenreId,
    pub audio_file_id: AudioFileId,
    pub size: i64,
    pub duration: i64,
}

#[derive(Debug, Clone)]
pub struct AudioFileDeleted {}

#[derive(Debug, Clone)]
pub struct AudioFileEvent {
    pub audio_file_id: AudioFileId,
    pub version: i64,
    pub kind: AudioFileEventKind,
}

#[derive(Debug, Clone)]
pub enum AudioFileEventKind {
    Created(AudioFileCreated),
    BoundToAlbum(AudioFileBoundToAlbum),
    ParticipantAdded(AudioFileParticipantAdded),
    ParticipantRemoved(AudioFileParticipantRemoved),
    GenreAdded(AudioFileGenreAdded),
    GenreRemoved(AudioFileGenreRemoved),
    UnboundFromAlbum(AudioFileUnboundFromAlbum),
    UnboundFromGenre(AudioFileUnboundFromGenre),
    Deleted(AudioFileDeleted),
    Updated(AudioFileUpdated),
}

impl DomainEvent for AudioFileEvent {
    fn aggregate_id(&self) -> i64 {
        self.audio_file_id.as_i64()
    }
    fn version(&self) -> i64 {
        self.version
    }
}

// ============== 仓储接口 ==============

/// AudioFileRepository 音频文件仓储接口
#[async_trait]
pub trait AudioFileRepository: Send + Sync {
    /// save 保存音频文件
    async fn save(&self, mut audio_file: AudioFile) -> Result<AudioFile, AudioFileError>;

    /// find_by_id 根据ID加载音频文件
    async fn find_by_id(&self, id: &AudioFileId) -> Result<Option<AudioFile>, AudioFileError>;

    /// find_by_path 根据路径加载音频文件
    async fn find_by_path(&self, path: &MediaPath) -> Result<Option<AudioFile>, AudioFileError>;

    /// delete 删除音频文件
    async fn delete(&self, id: &AudioFileId) -> Result<(), AudioFileError>;
}

/// AudioFileError 音频文件相关错误
#[derive(Error, Debug)]
pub enum AudioFileError {
    #[error("Audio file is currently being processed")]
    ProcessingInProgress,
    #[error("Audio file not found: {0:?}")]
    NotFound(AudioFileId),
    #[error("Audio file already exists: {0}")]
    AlreadyExists(String),
    #[error("Invalid audio file format: {0}")]
    InvalidFormat(String),
    #[error("Database error: {0}")]
    DbError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Audio file is already bound to a song")]
    AlreadyBoundToAlbum,
    #[error("Audio file is not bound to any album")]
    NotBoundToAlbum,
    #[error("Audio file is already bound to an artist")]
    AlreadyBoundToArtist,
    #[error("Audio file is not bound to any artist")]
    NotBoundToArtist,
    #[error("Audio file is already bound to a genre")]
    AlreadyBoundToGenre,
    #[error("Audio file is not bound to any genre")]
    NotBoundToGenre,
    #[error("Cannot delete file that is bound to a song")]
    CannotDeleteBoundFile,
    #[error("Invalid metadata: {0}")]
    InvalidMetadata(String),
    #[error("Participant not found: {0}")]
    ParticipantNotFound(Participant),
}
