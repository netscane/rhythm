use crate::query::dao::AudioFileDao;
use crate::query::QueryError;
use model::audio_file::AudioFile;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetSong {
    dao: Arc<dyn AudioFileDao + Send + Sync>,
}

impl GetSong {
    pub fn new(dao: Arc<dyn AudioFileDao + Send + Sync>) -> Self {
        Self { dao }
    }

    pub async fn handle(&self, song_id: i64) -> Result<AudioFile, QueryError> {
        let audio_file = self
            .dao
            .get_by_id(song_id)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        match audio_file {
            Some(audio_file) => Ok(audio_file),
            None => Err(QueryError::InvalidInput(format!(
                "Song not found: {}",
                song_id
            ))),
        }
    }
}

