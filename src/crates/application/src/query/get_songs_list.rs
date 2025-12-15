use crate::query::dao::AudioFileDao;
use crate::query::QueryError;
use model::audio_file::AudioFile;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetSongsList {
    audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
}

impl GetSongsList {
    pub fn new(audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>) -> Self {
        Self { audio_file_dao }
    }

    /// 根据类型获取歌曲列表
    /// - frequent/mostPlayed: 播放次数最多的歌曲
    /// - recent/recentlyPlayed: 最近播放的歌曲
    pub async fn handle(&self, list_type: &str, size: i32) -> Result<Vec<AudioFile>, QueryError> {
        let limit = size.min(500);
        
        match list_type.to_lowercase().as_str() {
            "frequent" | "mostplayed" => {
                self.audio_file_dao.get_most_played(limit).await
            }
            "recent" | "recentlyplayed" => {
                self.audio_file_dao.get_recently_played(limit).await
            }
            _ => Err(QueryError::InvalidParameter(format!(
                "Unknown list type: {}. Supported types: frequent, mostPlayed, recent, recentlyPlayed",
                list_type
            ))),
        }
    }
}
