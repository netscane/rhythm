use std::sync::Arc;

use super::shared::IdGenerator;
use crate::error::AppError;
use domain::play_queue::{PlayQueue, PlayQueueError, PlayQueueRepository};
use domain::value::{AudioFileId, PlayQueueId, UserId};

/// Save play queue command
#[derive(Debug)]
pub struct SavePlayQueueCmd {
    pub user_id: i64,
    /// List of audio file IDs in the queue
    pub song_ids: Vec<i64>,
    /// Current playing song ID
    pub current_id: Option<i64>,
    /// Position in milliseconds within the currently playing song
    pub position: i64,
    /// Client name
    pub changed_by: String,
}

/// Play queue application service
pub struct PlayQueueAppService {
    play_queue_repository: Arc<dyn PlayQueueRepository>,
    id_generator: Arc<dyn IdGenerator>,
}

impl PlayQueueAppService {
    pub fn new(
        play_queue_repository: Arc<dyn PlayQueueRepository>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            play_queue_repository,
            id_generator,
        }
    }

    /// Save play queue state
    ///
    /// If song_ids is empty, clears the play queue for the user.
    /// Otherwise, saves the play queue state.
    pub async fn save_play_queue(&self, cmd: SavePlayQueueCmd) -> Result<(), AppError> {
        let user_id = UserId::from(cmd.user_id);

        // If no songs, clear the queue
        if cmd.song_ids.is_empty() {
            self.play_queue_repository
                .delete_by_user_id(user_id)
                .await
                .map_err(|e| AppError::UnknownError(e.to_string()))?;
            return Ok(());
        }

        // Find existing play queue or create new one
        let existing = self
            .play_queue_repository
            .find_by_user_id(user_id.clone())
            .await
            .map_err(|e| AppError::UnknownError(e.to_string()))?;

        let play_queue_id = match existing {
            Some(pq) => pq.id,
            None => PlayQueueId::from(self.id_generator.next_id().await?),
        };

        // Build items list
        let items: Vec<AudioFileId> = cmd.song_ids.into_iter().map(AudioFileId::from).collect();

        // Build current audio file id
        let current = cmd.current_id.map(AudioFileId::from);

        // Create play queue from saved state
        let mut play_queue = PlayQueue::from_saved_state(
            play_queue_id,
            user_id,
            items,
            current,
            cmd.position,
            cmd.changed_by,
        );

        // Save
        self.play_queue_repository
            .save(&mut play_queue)
            .await
            .map_err(|e| AppError::UnknownError(e.to_string()))?;

        Ok(())
    }
}

impl From<PlayQueueError> for AppError {
    fn from(e: PlayQueueError) -> Self {
        match e {
            PlayQueueError::DbErr(msg) => AppError::RepositoryError("PlayQueue".to_string(), msg),
            PlayQueueError::VersionConflict { expected, actual } => AppError::UnknownError(
                format!("Version conflict: expected {}, got {}", expected, actual),
            ),
            PlayQueueError::OtherErr(msg) => AppError::UnknownError(msg),
        }
    }
}
