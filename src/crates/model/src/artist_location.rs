use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::value::{ArtistId, MediaPath};

#[derive(Debug, Clone)]
pub struct ArtistLocation {
    pub artist_id: ArtistId,
    pub location: MediaPath,
    pub total: i32,
    pub update_time: Option<NaiveDateTime>,
}

#[async_trait]
pub trait ArtistLocationRepository: Send + Sync {
    /// Adjust count for specific location (unified operation for both increment and decrement)
    /// The entry.total can be positive (increment) or negative (decrement)
    /// SQL: INSERT ... ON CONFLICT (artist_id, protocol, path) DO UPDATE SET total = total + entry.total
    async fn adjust_count(
        &self,
        entry: ArtistLocation,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
