use crate::query::dao::PlayQueueDao;
use crate::query::QueryError;
use model::play_queue::PlayQueue;
use std::sync::Arc;

/// 获取播放队列查询服务
#[derive(Clone)]
pub struct GetPlayQueue {
    play_queue_dao: Arc<dyn PlayQueueDao + Send + Sync>,
}

impl GetPlayQueue {
    pub fn new(play_queue_dao: Arc<dyn PlayQueueDao + Send + Sync>) -> Self {
        Self { play_queue_dao }
    }

    /// 根据用户 ID 获取播放队列（包含歌曲详情）
    pub async fn get_by_user_id(
        &self,
        user_id: i64,
        username: &str,
    ) -> Result<Option<PlayQueue>, QueryError> {
        self.play_queue_dao.get_by_user_id(user_id, username).await
    }
}
