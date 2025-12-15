use chrono::NaiveDateTime;
use std::fmt::{self, Display};

// Helper macro to define aggregate ID newtypes and common trait impls
macro_rules! define_id {
    ($name:ident $(, $extra:ident)*) => {
        #[derive(Debug, Clone, PartialEq $(, $extra)*)]
        pub struct $name(i64);

        impl $name {
            pub fn as_i64(&self) -> i64 {
                self.0
            }
        }

        impl From<i64> for $name {
            fn from(id: i64) -> Self {
                Self(id)
            }
        }

        impl From<$name> for i64 {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

    };
}

// 媒体文件统一资源标识符
#[derive(Debug, Clone, PartialEq)]
pub struct MediaPath {
    pub protocol: String, // 存储协议（如 "local", "smb"）
    pub path: String,
}

impl Default for MediaPath {
    fn default() -> Self {
        Self {
            protocol: String::new(),
            path: String::new(),
        }
    }
}

impl MediaPath {
    pub fn new(protocol: String, path: String) -> Self {
        Self { protocol, path }
    }
    pub fn is_local(&self) -> bool {
        self.protocol.is_empty() || self.protocol == "local"
    }
    pub fn parent_path(&self) -> MediaPath {
        let mut parts: Vec<&str> = self.path.split('/').collect();
        // Remove trailing empty segment if path ends with '/'
        if matches!(parts.last(), Some(&"")) {
            parts.pop();
        }
        // Remove the last segment as the "file/leaf"
        let _ = parts.pop();
        // Special-case: keep root if it started with '/'
        let path = if self.path.starts_with('/') {
            format!("/{}", parts.join("/"))
        } else {
            parts.join("/")
        };
        MediaPath {
            path,
            protocol: self.protocol.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaLocation {
    pub library_id: i64,
    pub path: MediaPath,
}

// 文件元数据
#[derive(Debug, Clone, PartialEq)]
pub struct FileMeta {
    pub path: MediaPath,      // 统一资源标识符
    pub dir_path: MediaPath,  // 目录URI
    pub size: i64,            // 文件大小
    pub suffix: String,       // 文件后缀
    pub mtime: NaiveDateTime, // 最后修改时间
    pub atime: NaiveDateTime, // 最后访问时间
    pub ctime: NaiveDateTime, // 创建时间
    pub hash: Option<String>, // 文件哈希值
}

impl FileMeta {
    pub fn new(
        path: MediaPath,
        dir_path: MediaPath,
        size: i64,
        suffix: String,
        mtime: NaiveDateTime,
        atime: NaiveDateTime,
        ctime: NaiveDateTime,
        hash: Option<String>,
    ) -> Self {
        Self {
            path,
            dir_path,
            size,
            suffix,
            mtime,
            atime,
            ctime,
            hash,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioMetadata {
    // 专辑相关信息
    pub album: String, // 专辑
    pub participants: Vec<ParticipantMeta>,
    pub genres: Vec<String>,       // 流派
    pub track_number: Option<i32>, // 在专辑中的曲目编号
    pub title: String,             // 歌曲标题

    // 发行信息
    pub year: Option<i32>, // 发行年份

    // 音频技术信息
    pub duration: i64,            // 音频时长（秒）
    pub bit_rate: i32,            // 比特率（kbps）
    pub sample_rate: i32,         // 采样率（Hz）
    pub channels: i32,            // 声道数
    pub picture: Option<Vec<u8>>, // 封面图片
    pub lyrics: Option<String>,   // 歌词内容
}

impl Default for AudioMetadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            participants: Vec::new(),
            album: String::new(),
            genres: Vec::new(),
            track_number: None,
            year: None,
            duration: 0,
            bit_rate: 0,
            sample_rate: 0,
            channels: 0,
            picture: None,
            lyrics: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum AudioQuality {
    Lossless,
    HiRes,
    Standard,
    Low,
}

impl fmt::Display for AudioQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Audio,
    Image,
    Nfo,
    Other,
}

impl From<FileType> for String {
    fn from(value: FileType) -> Self {
        match value {
            FileType::Audio => "audio".to_string(),
            FileType::Image => "image".to_string(),
            FileType::Nfo => "nfo".to_string(),
            FileType::Other => "other".to_string(),
        }
    }
}

impl TryFrom<String> for FileType {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "audio" => Ok(FileType::Audio),
            "image" => Ok(FileType::Image),
            "nfo" => Ok(FileType::Nfo),
            "other" => Ok(FileType::Other),
            _ => Err(format!("invalid value:{}", value)),
        }
    }
}

define_id!(AudioFileId, Eq, Hash);
define_id!(AlbumId, Eq, Hash);
define_id!(ArtistId, Eq, Hash);
define_id!(GenreId, Eq, Hash);
define_id!(PlaylistId);
define_id!(AnnotationId);
define_id!(TranscodingId);
define_id!(CoverArtId);
define_id!(LibraryId, Eq, Hash);
define_id!(LibraryItemId);
define_id!(UserId);
define_id!(PlayQueueId);

define_id!(PlayerId, Eq, Hash);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlbumArtist {
    pub album_id: AlbumId,
    pub artist_id: ArtistId,
}

impl AlbumArtist {
    pub fn new(album_id: AlbumId, artist_id: ArtistId) -> Self {
        Self {
            album_id,
            artist_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioFileArtist {
    pub audio_file_id: AudioFileId,
    pub artist_id: ArtistId,
}

impl AudioFileArtist {
    pub fn new(audio_file_id: AudioFileId, artist_id: ArtistId) -> Self {
        Self {
            audio_file_id,
            artist_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParticipantWorkType {
    Album,
    Artist,
}

impl Display for ParticipantWorkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParticipantWorkType::Album => write!(f, "Album"),
            ParticipantWorkType::Artist => write!(f, "Artist"),
        }
    }
}

impl From<String> for ParticipantWorkType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Album" => ParticipantWorkType::Album,
            "Artist" => ParticipantWorkType::Artist,
            _ => ParticipantWorkType::Album, // Default fallback
        }
    }
}

impl From<&str> for ParticipantWorkType {
    fn from(s: &str) -> Self {
        match s {
            "Album" => ParticipantWorkType::Album,
            "Artist" => ParticipantWorkType::Artist,
            _ => ParticipantWorkType::Album, // Default fallback
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParticipantRole {
    AlbumArtist,
    Artist,
    Performer,
}

impl Display for ParticipantRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<String> for ParticipantRole {
    fn from(s: String) -> Self {
        match s.as_str() {
            "AlbumArtist" => ParticipantRole::AlbumArtist,
            "Artist" => ParticipantRole::Artist,
            "Performer" => ParticipantRole::Performer,
            _ => ParticipantRole::AlbumArtist, // Default fallback
        }
    }
}

impl From<&str> for ParticipantRole {
    fn from(s: &str) -> Self {
        match s {
            "AlbumArtist" => ParticipantRole::AlbumArtist,
            "Artist" => ParticipantRole::Artist,
            "Performer" => ParticipantRole::Performer,
            _ => ParticipantRole::AlbumArtist, // Default fallback
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParticipantSubRole {
    Bass,
    Drums,
    Guitar,
    Keyboard,
    Vocals,
    Other,
}

impl Display for ParticipantSubRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<String> for ParticipantSubRole {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Bass" => ParticipantSubRole::Bass,
            "Drums" => ParticipantSubRole::Drums,
            "Guitar" => ParticipantSubRole::Guitar,
            "Keyboard" => ParticipantSubRole::Keyboard,
            "Vocals" => ParticipantSubRole::Vocals,
            "Other" => ParticipantSubRole::Other,
            _ => ParticipantSubRole::Other, // Default fallback
        }
    }
}

impl From<&str> for ParticipantSubRole {
    fn from(s: &str) -> Self {
        match s {
            "Bass" => ParticipantSubRole::Bass,
            "Drums" => ParticipantSubRole::Drums,
            "Guitar" => ParticipantSubRole::Guitar,
            "Keyboard" => ParticipantSubRole::Keyboard,
            "Vocals" => ParticipantSubRole::Vocals,
            "Other" => ParticipantSubRole::Other,
            _ => ParticipantSubRole::Other, // Default fallback
        }
    }
}

impl From<ParticipantSubRole> for String {
    fn from(value: ParticipantSubRole) -> Self {
        match value {
            ParticipantSubRole::Bass => "Bass".to_string(),
            ParticipantSubRole::Drums => "Drums".to_string(),
            ParticipantSubRole::Guitar => "Guitar".to_string(),
            ParticipantSubRole::Keyboard => "Keyboard".to_string(),
            ParticipantSubRole::Vocals => "Vocals".to_string(),
            ParticipantSubRole::Other => "Other".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParticipantMeta {
    pub role: ParticipantRole,
    pub sub_role: Option<ParticipantSubRole>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Participant {
    pub artist_id: ArtistId,
    pub role: ParticipantRole,
    pub sub_role: Option<ParticipantSubRole>,
    pub work_id: i64,
    pub work_type: ParticipantWorkType,
}

impl Display for Participant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Participant(artist_id: {}, role: {:?}, sub_role: {:?}, work_id: {}, work_type: {:?})",
            self.artist_id, self.role, self.sub_role, self.work_id, self.work_type
        )
    }
}

impl Participant {
    pub fn new(
        artist_id: ArtistId,
        role: ParticipantRole,
        sub_role: Option<ParticipantSubRole>,
        work_id: i64,
        work_type: ParticipantWorkType,
    ) -> Self {
        Self {
            artist_id,
            role,
            sub_role: sub_role.clone(),
            work_id,
            work_type,
        }
    }
}
