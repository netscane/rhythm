use crate::ModelError;
use domain::value::AlbumId;

#[derive(Clone)]
pub struct AlbumStats {
    pub album_id: AlbumId,
    pub duration: i64,
    pub size: i64,
    pub song_count: i32,
    pub disk_numbers: Vec<i32>,
    pub year: Option<i32>,
}

/// Entry for adjusting album stats incrementally
#[derive(Clone)]
pub struct AlbumStatsAdjustment {
    pub album_id: AlbumId,
    pub duration_delta: i64,      // Can be positive or negative
    pub size_delta: i64,           // Can be positive or negative
    pub song_count_delta: i32,     // Can be positive or negative
    pub disk_number: Option<i32>,  // Disk number to add (if Some)
    pub year: Option<i32>,         // Year to set (if Some and not already set)
}

use async_trait::async_trait;

#[async_trait]
pub trait AlbumStatsRepository: Send + Sync {
    /// Adjust album stats incrementally (unified operation for both increment and decrement)
    /// The delta values can be positive (increment) or negative (decrement)
    async fn adjust_stats(&self, adjustment: AlbumStatsAdjustment) -> Result<(), ModelError>;
    
    /// Legacy methods (kept for backward compatibility)
    async fn find_by_album_id(&self, album_id: AlbumId) -> Result<Option<AlbumStats>, ModelError>;
    async fn save(&self, album_stats: AlbumStats) -> Result<(), ModelError>;
    async fn delete_by_album_id(&self, album_id: AlbumId) -> Result<(), ModelError>;
}
