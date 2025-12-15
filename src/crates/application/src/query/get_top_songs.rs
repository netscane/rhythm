use crate::query::dao::{ArtistDao, AudioFileDao};
use crate::query::QueryError;
use model::audio_file::AudioFile;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetTopSongs {
    artist_dao: Arc<dyn ArtistDao + Send + Sync>,
    audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
}

impl GetTopSongs {
    pub fn new(
        artist_dao: Arc<dyn ArtistDao + Send + Sync>,
        audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
    ) -> Self {
        Self {
            artist_dao,
            audio_file_dao,
        }
    }

    /// 根据艺术家名称查询 top songs
    /// 首先通过 sort_name 在 DAO 层查找艺术家，然后使用艺术家 ID 查询 top songs（按播放次数排序）
    pub async fn handle(
        &self,
        artist_name: &str,
        limit: i32,
    ) -> Result<Vec<AudioFile>, QueryError> {
        // 在 DAO 层查询艺术家（优先匹配 sort_name，如果没有则匹配 name）
        let artist = self
            .artist_dao
            .get_by_sort_name(artist_name)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        match artist {
            Some(artist) => {
                // 使用艺术家 ID 查询 top songs
                self.audio_file_dao
                    .get_top_songs_by_artist_id(artist.id, limit)
                    .await
                    .map_err(|e| QueryError::ExecutionError(e.to_string()))
            }
            None => Ok(Vec::new()), // 艺术家不存在，返回空列表
        }
    }
}
