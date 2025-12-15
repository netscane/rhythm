use crate::query::dao::PlaylistDao;
use crate::query::QueryError;
use model::playlist::{Playlist, PlaylistSummary};
use std::sync::Arc;

/// 获取播放列表查询服务
#[derive(Clone)]
pub struct GetPlaylist {
    playlist_dao: Arc<dyn PlaylistDao + Send + Sync>,
}

impl GetPlaylist {
    pub fn new(playlist_dao: Arc<dyn PlaylistDao + Send + Sync>) -> Self {
        Self { playlist_dao }
    }

    /// 根据 ID 获取播放列表（包含歌曲详情）
    pub async fn get_by_id(&self, playlist_id: i64) -> Result<Playlist, QueryError> {
        self.playlist_dao
            .get_by_id(playlist_id)
            .await?
            .ok_or_else(|| QueryError::InvalidInput(format!("Playlist not found: {}", playlist_id)))
    }

    /// 根据用户 ID 获取播放列表列表（基本信息）
    pub async fn get_by_owner_id(&self, owner_id: i64) -> Result<Vec<PlaylistSummary>, QueryError> {
        self.playlist_dao.get_by_owner_id(owner_id).await
    }
}
