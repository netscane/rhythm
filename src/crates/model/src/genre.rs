use crate::ModelError;
use async_trait::async_trait;
use domain::value::GenreId;

#[derive(Debug, Clone)]
pub struct Genre {
    pub name: String,
    pub song_count: i32,
    pub album_count: i32,
}

#[derive(Debug, Clone)]
pub struct GenreStats {
    pub genre_id: GenreId,
    pub song_count: i32,
    pub album_count: i32,
}

#[async_trait]
pub trait GenreStatsRepository: Send + Sync {
    /// Adjust genre stats (unified operation for both increment and decrement)
    /// The entry values can be positive (increment) or negative (decrement)
    async fn adjust_stats(&self, entry: GenreStats) -> Result<(), ModelError>;
}
