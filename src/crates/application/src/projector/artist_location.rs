use crate::{command::shared::IdGenerator, error::AppError};
use domain::audio_file::AudioFileEvent;
use domain::audio_file::AudioFileEventKind;
use domain::value::ArtistId;
use domain::value::MediaPath;
use model::artist_location::{ArtistLocation, ArtistLocationRepository};
use std::sync::Arc;

pub struct ArtistLocationProjector {
    artist_location_repository: Arc<dyn ArtistLocationRepository>,
    id_generator: Arc<dyn IdGenerator>,
}

impl ArtistLocationProjector {
    pub fn new(
        artist_location_repository: Arc<dyn ArtistLocationRepository>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            artist_location_repository,
            id_generator,
        }
    }

    pub async fn on_audio_file_participant_added(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        if let AudioFileEventKind::ParticipantAdded(evt_kind) = &event.kind {
            let participant = evt_kind.participant.clone();
            self.update_artist_location(participant.artist_id, &evt_kind.path)
                .await?;
        }
        Ok(())
    }

    pub async fn on_audio_file_participant_removed(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        if let AudioFileEventKind::ParticipantRemoved(evt_kind) = &event.kind {
            let participant = evt_kind.participant.clone();
            self.remove_audio_file_from_artist_location(participant.artist_id, &evt_kind.path)
                .await?;
        }
        Ok(())
    }

    async fn update_artist_location(
        &self,
        artist_id: ArtistId,
        audio_file_path: &MediaPath,
    ) -> Result<(), AppError> {
        let dir_path = audio_file_path.parent_path();

        // Optimized: Single adjust operation
        // INSERT ... ON CONFLICT (artist_id, protocol, path) DO UPDATE SET total = total + 1
        let entry = ArtistLocation {
            artist_id,
            location: dir_path,
            total: 1, // Increment by 1
            update_time: None, // Will be set by repository
        };

        self.artist_location_repository
            .adjust_count(entry)
            .await
            .map_err(|e| AppError::ProjectionError(e.to_string()))?;

        Ok(())
    }

    async fn remove_audio_file_from_artist_location(
        &self,
        artist_id: ArtistId,
        audio_file_path: &MediaPath,
    ) -> Result<(), AppError> {
        let dir_path = audio_file_path.parent_path();

        // Optimized: Single adjust operation
        let entry = ArtistLocation {
            artist_id,
            location: dir_path,
            total: -1, // Decrement by 1
            update_time: None, // Will be set by repository
        };

        self.artist_location_repository
            .adjust_count(entry)
            .await
            .map_err(|e| AppError::ProjectionError(e.to_string()))?;

        Ok(())
    }
}
