use crate::event::event_bus::{EventEnvelope, Handler};
use crate::projector::album_stats::AlbumStatsProjector;
use domain::audio_file::{AudioFileEvent, AudioFileEventKind};
use log::{debug, error};

pub struct AlbumStatsHandler {
    album_stats_projector: AlbumStatsProjector,
}

impl AlbumStatsHandler {
    pub fn new(album_stats_projector: AlbumStatsProjector) -> Self {
        Self {
            album_stats_projector,
        }
    }
}

#[async_trait::async_trait]
impl Handler<AudioFileEvent> for AlbumStatsHandler {
    async fn handle(&self, event_envelope: &EventEnvelope<AudioFileEvent>) {
        match &event_envelope.payload.kind {
            AudioFileEventKind::BoundToAlbum(_) => {
                if let Err(e) = self
                    .album_stats_projector
                    .on_audio_file_bound_to_album(&event_envelope.payload)
                    .await
                {
                    error!("Failed to handle audio file bound to album event: {}", e);
                }
            }
            AudioFileEventKind::UnboundFromAlbum(_) => {
                if let Err(e) = self
                    .album_stats_projector
                    .on_audio_file_unbound_from_album(&event_envelope.payload)
                    .await
                {
                    error!(
                        "Failed to handle audio file unbound from album event: {}",
                        e
                    );
                }
            }
            _ => {
                // 其他事件不需要处理
                debug!("Audio file event received, no action needed for album stat");
            }
        }
    }
}
