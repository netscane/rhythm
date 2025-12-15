use crate::{command::shared::IdGenerator, error::AppError};
use domain::audio_file::AudioFileEvent;
use domain::audio_file::AudioFileEventKind;
use domain::value::AlbumId;
use domain::value::MediaPath;
use model::album_location::{AlbumLocation, AlbumLocationRepository};
use std::sync::Arc;

pub struct AlbumLocationProjector {
    album_location_repository: Arc<dyn AlbumLocationRepository>,
    id_generator: Arc<dyn IdGenerator>,
}

impl AlbumLocationProjector {
    pub fn new(
        album_location_repository: Arc<dyn AlbumLocationRepository>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            album_location_repository,
            id_generator,
        }
    }

    pub async fn on_audio_file_album_id_updated(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        if let AudioFileEventKind::BoundToAlbum(evt_kind) = &event.kind {
            self.update_album_location(evt_kind.album_id.clone(), &evt_kind.path)
                .await?;
        }
        Ok(())
    }

    pub async fn on_audio_file_album_id_removed(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        if let AudioFileEventKind::UnboundFromAlbum(evt_kind) = &event.kind {
            self.remove_audio_file_from_album_location(evt_kind.album_id.clone(), &evt_kind.path)
                .await?;
        }
        Ok(())
    }

    async fn update_album_location(
        &self,
        album_id: AlbumId,
        audio_file_path: &MediaPath,
    ) -> Result<(), AppError> {
        let dir_path = audio_file_path.parent_path();

        // Optimized: Single adjust operation
        // INSERT ... ON CONFLICT (album_id, protocol, path) DO UPDATE SET count = count + 1
        let entry = AlbumLocation {
            album_id,
            location: dir_path,
            total: 1, // Increment by 1
            update_time: None, // Will be set by repository
        };

        self.album_location_repository
            .adjust_count(entry)
            .await
            .map_err(|e| AppError::ProjectionError(e.to_string()))?;

        Ok(())
    }

    async fn remove_audio_file_from_album_location(
        &self,
        album_id: AlbumId,
        audio_file_path: &MediaPath,
    ) -> Result<(), AppError> {
        let dir_path = audio_file_path.parent_path();

        // Optimized: Single adjust operation
        let entry = AlbumLocation {
            album_id,
            location: dir_path,
            total: -1, // Decrement by 1
            update_time: None, // Will be set by repository
        };

        self.album_location_repository
            .adjust_count(entry)
            .await
            .map_err(|e| AppError::ProjectionError(e.to_string()))?;

        Ok(())
    }
}
