use crate::ModelError;
use chrono::NaiveDateTime;
use domain::value::{AlbumId, MediaPath};

#[derive(Clone, Debug)]
pub struct AlbumLocation {
    pub album_id: AlbumId,
    pub location: MediaPath,
    pub total: i32,
    pub update_time: Option<NaiveDateTime>,
}

use async_trait::async_trait;

#[async_trait]
pub trait AlbumLocationRepository: Send + Sync {
    /// Adjust count for specific location (unified operation for both increment and decrement)
    /// The entry.total can be positive (increment) or negative (decrement)
    /// SQL: INSERT ... ON CONFLICT (album_id, protocol, path) DO UPDATE SET count = count + entry.total
    async fn adjust_count(&self, entry: AlbumLocation) -> Result<(), ModelError>;
}
