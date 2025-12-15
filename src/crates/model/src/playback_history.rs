use crate::ModelError;
use async_trait::async_trait;
use domain::value::{AudioFileId, PlayerId, UserId};

use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct PlaybackHistoryEntry {
    pub user_id: UserId,
    pub audio_file_id: AudioFileId,
    pub scrobbled_at: NaiveDateTime,
}

#[async_trait]
pub trait PlaybackHistoryRepository: Send + Sync {
    async fn save(&self, entry: &PlaybackHistoryEntry) -> Result<(), ModelError>;
}
