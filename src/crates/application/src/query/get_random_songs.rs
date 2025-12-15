use crate::query::dao::AudioFileDao;
use crate::query::QueryError;
use model::audio_file::AudioFile;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetRandomSongs {
    audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
}

impl GetRandomSongs {
    pub fn new(audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>) -> Self {
        Self { audio_file_dao }
    }

    pub async fn handle(
        &self,
        genre: Option<&str>,
        from_year: Option<i32>,
        to_year: Option<i32>,
        size: i32,
    ) -> Result<Vec<AudioFile>, QueryError> {
        let limit = size.min(500); // 限制最大值为 500
        self.audio_file_dao
            .get_random_songs(genre, from_year, to_year, limit)
            .await
    }
}

