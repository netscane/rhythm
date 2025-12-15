use crate::event::event_bus::{EventEnvelope, Handler};
use crate::projector::genre_stats::GenreStatsProjector;
use domain::album::{AlbumEvent, AlbumEventKind};
use domain::audio_file::{AudioFileEvent, AudioFileEventKind};
use log::{debug, error};

pub struct GenreStatsHandler {
    genre_stats_projector: GenreStatsProjector,
}

impl GenreStatsHandler {
    pub fn new(genre_stats_projector: GenreStatsProjector) -> Self {
        Self {
            genre_stats_projector,
        }
    }
}

#[async_trait::async_trait]
impl Handler<AudioFileEvent> for GenreStatsHandler {
    async fn handle(&self, event_envelope: &EventEnvelope<AudioFileEvent>) {
        match &event_envelope.payload.kind {
            AudioFileEventKind::GenreAdded(_) => {
                if let Err(e) = self
                    .genre_stats_projector
                    .on_audio_file_bound_to_genre(&event_envelope.payload)
                    .await
                {
                    error!("Failed to handle audio file bound to genre event: {}", e);
                }
            }
            AudioFileEventKind::GenreRemoved(_) => {
                if let Err(e) = self
                    .genre_stats_projector
                    .on_audio_file_unbound_from_genre(&event_envelope.payload)
                    .await
                {
                    error!(
                        "Failed to handle audio file unbound from genre event: {}",
                        e
                    );
                }
            }
            _ => {
                // 其他事件不需要处理
                debug!("Audio file event received, no action needed for genre stats");
            }
        }
    }
}

#[async_trait::async_trait]
impl Handler<AlbumEvent> for GenreStatsHandler {
    async fn handle(&self, event_envelope: &EventEnvelope<AlbumEvent>) {
        match &event_envelope.payload.kind {
            AlbumEventKind::BoundToGenre(_) => {
                if let Err(e) = self
                    .genre_stats_projector
                    .on_album_genre_added(&event_envelope.payload)
                    .await
                {
                    error!("Failed to handle album genre added event: {}", e);
                }
            }
            AlbumEventKind::UnboundFromGenre(_) => {
                if let Err(e) = self
                    .genre_stats_projector
                    .on_album_genre_removed(&event_envelope.payload)
                    .await
                {
                    error!("Failed to handle album genre removed event: {}", e);
                }
            }
            _ => {
                // 其他事件不需要处理
                debug!("Album event received, no action needed for genre stats");
            }
        }
    }
}
