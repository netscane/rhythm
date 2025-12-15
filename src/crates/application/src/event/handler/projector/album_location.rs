use crate::event::event_bus::{EventEnvelope, Handler};
use crate::projector::album_location::AlbumLocationProjector;
use domain::audio_file::{AudioFileEvent, AudioFileEventKind};
use log::{debug, error};

pub struct AlbumLocationHandler {
    projector: AlbumLocationProjector,
}

impl AlbumLocationHandler {
    pub fn new(projector: AlbumLocationProjector) -> Self {
        Self { projector }
    }
}

#[async_trait::async_trait]
impl Handler<AudioFileEvent> for AlbumLocationHandler {
    async fn handle(&self, event: &EventEnvelope<AudioFileEvent>) {
        match &event.payload.kind {
            AudioFileEventKind::BoundToAlbum(_) => {
                if let Err(e) = self
                    .projector
                    .on_audio_file_album_id_updated(&event.payload)
                    .await
                {
                    error!("Failed to handle audio file bound to album event: {}", e);
                }
            }
            AudioFileEventKind::UnboundFromAlbum(_) => {
                if let Err(e) = self
                    .projector
                    .on_audio_file_album_id_removed(&event.payload)
                    .await
                {
                    error!("Failed to handle audio file unbound from album event: {}", e);
                }
            }
            _ => {
                debug!("Audio file event received, no action needed for album location");
            }
        }
    }
}
