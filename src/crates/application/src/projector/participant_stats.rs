use crate::command::shared::IdGenerator;
use crate::error::AppError;
use dashmap::DashMap;
use domain::album::{AlbumEvent, AlbumEventKind};
use domain::audio_file::{AudioFileEvent, AudioFileEventKind};
use domain::value::{ArtistId, ParticipantRole};
use model::participant_stats::{ParticipantStats, ParticipantStatsRepository};
use std::sync::Arc;
use tokio::sync::Mutex;

/// 锁键，用于标识需要加锁的 (artist_id, role) 组合
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LockKey {
    artist_id: ArtistId,
    role: ParticipantRole,
}

/// ArtistStatProjector 处理音频文件和专辑的绑定/解绑事件，更新艺术家统计信息
pub struct ParticipantStatsProjector {
    participant_stats_repository: Arc<dyn ParticipantStatsRepository>,
    id_generator: Arc<dyn IdGenerator>,
    /// 为每个 (artist_id, role) 组合维护一个锁，防止并发更新冲突
    locks: Arc<DashMap<LockKey, Arc<Mutex<()>>>>,
}

impl ParticipantStatsProjector {
    pub fn new(
        participant_stats_repository: Arc<dyn ParticipantStatsRepository>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            participant_stats_repository,
            id_generator,
            locks: Arc::new(DashMap::new()),
        }
    }

    /// 获取指定 (artist_id, role) 的锁
    fn get_lock(&self, artist_id: &ArtistId, role: &ParticipantRole) -> Arc<Mutex<()>> {
        let key = LockKey {
            artist_id: artist_id.clone(),
            role: role.clone(),
        };
        // DashMap::entry 的 or_insert_with 是原子操作，确保只创建一个锁
        self.locks
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// 处理音频文件绑定到艺术家事件
    pub async fn on_audio_file_participant_added(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        if let AudioFileEventKind::ParticipantAdded(evt_kind) = &event.kind {
            let entry = ParticipantStats {
                artist_id: evt_kind.participant.artist_id.clone(),
                role: evt_kind.participant.role.clone(),
                duration: evt_kind.duration as i64, // Add duration
                size: evt_kind.size,                // Add size
                song_count: 1,                      // Increment song count
                album_count: 0,
            };

            self.participant_stats_repository
                .adjust_participant_stats(entry)
                .await?;
        }
        Ok(())
    }

    /// 处理音频文件从艺术家解绑事件
    pub async fn on_audio_file_participant_removed(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        if let AudioFileEventKind::ParticipantRemoved(evt_kind) = &event.kind {
            let entry = ParticipantStats {
                artist_id: evt_kind.participant.artist_id.clone(),
                role: evt_kind.participant.role.clone(),
                duration: -(evt_kind.duration as i64), // Subtract duration
                size: -(evt_kind.size as i64),         // Subtract size
                song_count: -1,                        // Decrement song count
                album_count: 0,
            };

            self.participant_stats_repository
                .adjust_participant_stats(entry)
                .await?;
        }
        Ok(())
    }

    pub async fn on_album_participant_added(&self, event: &AlbumEvent) -> Result<(), AppError> {
        if let AlbumEventKind::ParticipantAdded(evt_kind) = &event.kind {
            let entry = ParticipantStats {
                artist_id: evt_kind.participant.artist_id.clone(),
                role: evt_kind.participant.role.clone(),
                duration: 0,
                size: 0,
                song_count: 0,
                album_count: 1, // Increment album count
            };

            self.participant_stats_repository
                .adjust_participant_stats(entry)
                .await?;
        }
        Ok(())
    }

    pub async fn on_album_participant_removed(&self, event: &AlbumEvent) -> Result<(), AppError> {
        if let AlbumEventKind::ParticipantRemoved(evt_kind) = &event.kind {
            let entry = ParticipantStats {
                artist_id: evt_kind.participant.artist_id.clone(),
                role: evt_kind.participant.role.clone(),
                duration: 0,
                size: 0,
                song_count: 0,
                album_count: -1, // Decrement album count
            };

            self.participant_stats_repository
                .adjust_participant_stats(entry)
                .await?;
        }
        Ok(())
    }

    /// 处理音频文件添加参与者事件（保持向后兼容）
    pub async fn handle_audio_file_participant_added(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        self.on_audio_file_participant_added(event).await
    }

    /// 处理音频文件移除参与者事件（保持向后兼容）
    pub async fn handle_audio_file_participant_removed(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        self.on_audio_file_participant_removed(event).await
    }

    /// 处理专辑添加参与者事件（保持向后兼容）
    pub async fn handle_album_participant_added(&self, event: &AlbumEvent) -> Result<(), AppError> {
        self.on_album_participant_added(event).await
    }

    /// 处理专辑移除参与者事件（保持向后兼容）
    pub async fn handle_album_participant_removed(
        &self,
        event: &AlbumEvent,
    ) -> Result<(), AppError> {
        self.on_album_participant_removed(event).await
    }
}
