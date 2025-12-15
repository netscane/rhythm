use crate::query::dao::AudioFileDao;
use crate::query::QueryError;
use model::audio_file::AudioFile;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetSongsByGenre {
    audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
}

impl GetSongsByGenre {
    pub fn new(audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>) -> Self {
        Self { audio_file_dao }
    }

    pub async fn handle(
        &self,
        genre: &str,
        offset: i32,
        count: i32,
    ) -> Result<Vec<AudioFile>, QueryError> {
        let limit = count.min(500); // 限制最大值为 500
        self.audio_file_dao.get_by_genre(genre, offset, limit).await
    }
}

