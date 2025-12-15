use crate::ModelError;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ArtistStats {
    pub album_count: i32,
    pub song_count: i32,
    pub size: i64,
}

#[derive(Debug)]
pub struct Artist {
    pub id: i64,
    pub name: String,
    pub sort_name: String,
    pub order_name: String,

    // stat
    pub size: i64,
    pub album_count: i32,
    pub song_count: i32,

    // annotation
    pub played_count: i32,
    pub played_at: Option<NaiveDateTime>,
    pub rating: i32,
    pub starred: bool,
    pub starred_at: Option<NaiveDateTime>,

    // metadata
    pub updated_at: Option<NaiveDateTime>,

    // reserved
    pub mbz_artist_id: Option<String>,
    // roles: role -> ArtistStats
    pub roles: HashMap<String, ArtistStats>,
}

#[derive(Debug)]
pub struct ArtistInfo {
    pub id: i64,
    pub name: String,
    pub biography: Option<String>,
}

#[async_trait]
pub trait ArtistRepository {
    async fn count_all(&self) -> Result<i64, ModelError>;
    async fn get_all(&self) -> Result<Vec<Artist>, ModelError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Artist>, ModelError>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Artist>, ModelError>;
    async fn list_by_album_id(&self, album_id: i64) -> Result<Vec<Artist>, ModelError>;
    async fn list_by_audio_file_id(&self, audio_file_id: i64) -> Result<Vec<Artist>, ModelError>;
}
