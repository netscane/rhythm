use crate::error::AppError;
use chrono::Local;
use domain::value::{AudioFileId, UserId};
use model::playback_history::{PlaybackHistoryEntry, PlaybackHistoryRepository};
use std::sync::Arc;

pub struct PlaybackHistoryProjector {
    repository: Arc<dyn PlaybackHistoryRepository>,
}

impl PlaybackHistoryProjector {
    pub fn new(repository: Arc<dyn PlaybackHistoryRepository>) -> Self {
        Self { repository }
    }

    /// 处理播放开始事件
    pub async fn on_scrobble(&self, item_id: AudioFileId, user_id: UserId) -> Result<(), AppError> {
        let entry = PlaybackHistoryEntry {
            user_id: user_id.clone(),
            audio_file_id: item_id.clone(),
            scrobbled_at: Local::now().naive_local(),
        };
        self.repository.save(&entry).await?;
        Ok(())
    }
}
