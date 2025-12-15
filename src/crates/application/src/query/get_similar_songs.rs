use crate::query::dao::{ArtistDao, AudioFileDao};
use crate::query::QueryError;
use model::audio_file::AudioFile;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetSimilarSongs {
    artist_dao: Arc<dyn ArtistDao + Send + Sync>,
    audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
}

impl GetSimilarSongs {
    pub fn new(
        artist_dao: Arc<dyn ArtistDao + Send + Sync>,
        audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
    ) -> Self {
        Self {
            artist_dao,
            audio_file_dao,
        }
    }

    /// 根据输入的 artist_id 查询相似艺术家的歌曲
    /// 暂时返回数据库中随机艺术家的歌曲
    pub async fn handle(&self, artist_id: i64, limit: i32) -> Result<Vec<AudioFile>, QueryError> {
        // 首先验证输入的 artist_id 是否存在
        let input_artist = self
            .artist_dao
            .get_by_id(artist_id)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        if input_artist.is_none() {
            return Err(QueryError::InvalidInput(format!(
                "Artist not found: {}",
                artist_id
            )));
        }

        // 查询相似艺术家 ID（暂时返回随机艺术家 ID，排除当前艺术家）
        let similar_artist_id = self
            .artist_dao
            .get_random_artist_id(Some(artist_id))
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        match similar_artist_id {
            Some(similar_id) => {
                // 查询相似艺术家的歌曲
                self.audio_file_dao
                    .get_by_artist_id(similar_id)
                    .await
                    .map_err(|e| QueryError::ExecutionError(e.to_string()))
                    .map(|songs| {
                        // 限制返回数量
                        songs.into_iter().take(limit as usize).collect()
                    })
            }
            None => Ok(Vec::new()), // 没有找到相似艺术家，返回空列表
        }
    }
}
