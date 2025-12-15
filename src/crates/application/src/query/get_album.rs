use crate::query::dao::{AlbumDao, AudioFileDao};
use crate::query::QueryError;
use model::album::Album;
use model::audio_file::AudioFile;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetAlbum {
    album_dao: Arc<dyn AlbumDao + Send + Sync>,
    audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
}

impl GetAlbum {
    pub fn new(
        album_dao: Arc<dyn AlbumDao + Send + Sync>,
        audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
    ) -> Self {
        Self {
            album_dao,
            audio_file_dao,
        }
    }

    pub async fn handle(&self, album_id: i64) -> Result<(Album, Vec<AudioFile>), QueryError> {
        let album = self
            .album_dao
            .get_by_id(album_id)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        let album = match album {
            Some(album) => album,
            None => {
                return Err(QueryError::InvalidInput(format!(
                    "Album not found: {}",
                    album_id
                )));
            }
        };

        let audio_files = self
            .audio_file_dao
            .get_by_album_id(album_id)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        Ok((album, audio_files))
    }
}

