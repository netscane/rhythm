use serde::{Deserialize, Serialize};

/// 播放列表完整信息（包含歌曲详情）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub comment: Option<String>,
    pub duration: i32,
    pub song_count: i32,
    pub owner_name: String,
    pub owner_id: i64,
    pub public: bool,
    pub tracks: Vec<PlaylistTrack>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 播放列表基本信息（不包含歌曲详情，用于列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistSummary {
    pub id: i64,
    pub name: String,
    pub comment: Option<String>,
    pub duration: i32,
    pub song_count: i32,
    pub owner_name: String,
    pub owner_id: i64,
    pub public: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 播放列表中的歌曲
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistTrack {
    pub id: i64,
    pub audio_file: PlaylistAudioFile,
}

/// 播放列表中的音频文件信息（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistAudioFile {
    pub id: i64,
    pub title: String,
    pub album_id: Option<i64>,
    pub album_name: Option<String>,
    pub artist_id: Option<i64>,
    pub artist_name: Option<String>,
    pub duration: Option<i64>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub bit_rate: Option<i32>,
    pub size: Option<i64>,
    pub suffix: Option<String>,
    pub content_type: Option<String>,
    pub path: String,
    pub cover_art_id: Option<i64>,
    pub play_count: i32,
    pub starred: Option<bool>,
    pub rating: Option<i32>,
    pub created_at: i64,
}
