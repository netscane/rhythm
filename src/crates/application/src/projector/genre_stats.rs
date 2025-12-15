use crate::error::AppError;
use domain::album::{AlbumEvent, AlbumEventKind};
use domain::audio_file::{AudioFileEvent, AudioFileEventKind};
use model::genre::{GenreStats, GenreStatsRepository};
use std::sync::Arc;

/// AlbumStatProjector 处理音频文件绑定/解绑事件，更新专辑统计信息
pub struct GenreStatsProjector {
    genre_stats_repository: Arc<dyn GenreStatsRepository>,
}

impl GenreStatsProjector {
    pub fn new(genre_stats_repository: Arc<dyn GenreStatsRepository>) -> Self {
        Self {
            genre_stats_repository,
        }
    }

    /// 处理音频文件绑定到流派事件
    pub async fn on_audio_file_bound_to_genre(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        if let AudioFileEventKind::GenreAdded(evt_kind) = &event.kind {
            let entry = GenreStats {
                genre_id: evt_kind.genre_id.clone(),
                song_count: 1, // Increment by 1
                album_count: 0,
            };

            self.genre_stats_repository
                .adjust_stats(entry)
                .await?;
        }
        Ok(())
    }

    /// 处理音频文件从流派解绑事件
    pub async fn on_audio_file_unbound_from_genre(
        &self,
        event: &AudioFileEvent,
    ) -> Result<(), AppError> {
        if let AudioFileEventKind::GenreRemoved(evt_kind) = &event.kind {
            let entry = GenreStats {
                genre_id: evt_kind.genre_id.clone(),
                song_count: -1, // Decrement by 1
                album_count: 0,
            };

            self.genre_stats_repository
                .adjust_stats(entry)
                .await?;
        }
        Ok(())
    }

    pub async fn on_album_genre_added(&self, event: &AlbumEvent) -> Result<(), AppError> {
        if let AlbumEventKind::BoundToGenre(evt_kind) = &event.kind {
            let entry = GenreStats {
                genre_id: evt_kind.genre_id.clone(),
                song_count: 0,
                album_count: 1, // Increment by 1
            };

            self.genre_stats_repository
                .adjust_stats(entry)
                .await?;
        }
        Ok(())
    }

    pub async fn on_album_genre_removed(&self, event: &AlbumEvent) -> Result<(), AppError> {
        if let AlbumEventKind::UnboundFromGenre(evt_kind) = &event.kind {
            let entry = GenreStats {
                genre_id: evt_kind.genre_id.clone(),
                song_count: 0,
                album_count: -1, // Decrement by 1
            };

            self.genre_stats_repository
                .adjust_stats(entry)
                .await?;
        }
        Ok(())
    }
}
