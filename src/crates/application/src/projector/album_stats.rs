use crate::error::AppError;
use domain::audio_file::{AudioFileEvent, AudioFileEventKind};
use domain::value::AlbumId;
use model::album_stats::{AlbumStats, AlbumStatsAdjustment, AlbumStatsRepository};
use std::sync::Arc;

/// AlbumStatProjector 处理音频文件绑定/解绑事件，更新专辑统计信息
pub struct AlbumStatsProjector {
    album_stats_repository: Arc<dyn AlbumStatsRepository>,
}

impl AlbumStatsProjector {
    pub fn new(album_stats_repository: Arc<dyn AlbumStatsRepository>) -> Self {
        Self {
            album_stats_repository,
        }
    }

    /// 处理音频文件绑定到专辑事件
    pub async fn on_audio_file_bound_to_album(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        if let AudioFileEventKind::BoundToAlbum(evt_kind) = &event.kind {
            let adjustment = AlbumStatsAdjustment {
                album_id: evt_kind.album_id.clone(),
                duration_delta: evt_kind.duration,
                size_delta: evt_kind.size,
                song_count_delta: 1,
                disk_number: evt_kind.disc_number,
                year: evt_kind.year,
            };

            self.album_stats_repository
                .adjust_stats(adjustment)
                .await?;
        }
        Ok(())
    }

    /// 处理音频文件从专辑解绑事件
    pub async fn on_audio_file_unbound_from_album(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        if let AudioFileEventKind::UnboundFromAlbum(evt_kind) = &event.kind {
            let adjustment = AlbumStatsAdjustment {
                album_id: evt_kind.album_id.clone(),
                duration_delta: -evt_kind.duration,
                size_delta: -evt_kind.size,
                song_count_delta: -1,
                disk_number: None, // Don't remove disk numbers on unbind
                year: None,        // Don't change year on unbind
            };

            self.album_stats_repository
                .adjust_stats(adjustment)
                .await?;
        }
        Ok(())
    }

    /// 处理音频文件绑定到专辑事件（保持向后兼容）
    pub async fn handle_audio_file_bound_to_album(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        self.on_audio_file_bound_to_album(event).await
    }

    /// 处理音频文件从专辑解绑事件（保持向后兼容）
    pub async fn handle_audio_file_unbound_from_album(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        self.on_audio_file_unbound_from_album(event).await
    }
}
