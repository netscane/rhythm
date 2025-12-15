use crate::commands::cover_art::{CoverArtService, CreateCoverArtCmd};
use crate::commands::event::{EventEnvelope, Handler};
use domain::audio_file::{AudioFileEvent, AudioFileEventKind};
use domain::library::FileType;
use domain::library::LibraryEvent;
use log::error;
use std::sync::Arc;
pub struct CoverArtHandler {
    cover_art_service: CoverArtService<B>,
}

impl CoverArtHandler {
    pub fn new(cover_art_service: CoverArtService<B>) -> Self {
        Self { cover_art_service }
    }
}

#[async_trait::async_trait]
impl Handler<AudioFileEvent> for CoverArtHandler {
    async fn handle(&self, envelope: &EventEnvelope<AudioFileEvent>) {
        let evt = &envelope.payload;
        match &evt.kind {
            AudioFileEventKind::Created(evt) => {
                let cmd = CreateCoverArtCmd {
                    media_path: evt.path.clone(),
                    file_type: FileType::Audio,
                    correlation_id: generate_correlation_id(),
                };
                if let Err(e) = self.cover_art_service.create_cover_art(cmd).await {
                    error!("Failed to create cover art, error:{}", e);
                    return;
                }
                return;
            }
            _ => return,
        }
    }
}

#[async_trait::async_trait]
impl Handler<LibraryEvent> for CoverArtHandler {
    async fn handle(&self, envelope: &EventEnvelope<LibraryEvent>) {
        let evt = &envelope.payload;
        match evt {
            LibraryEvent::FileAdded(evt) => {
                if evt.item.file_type != FileType::Image {
                    return;
                }
                let cmd = CreateCoverArtCmd {
                    media_path: evt.item.path.clone(),
                    file_type: FileType::Image,
                };
                if let Err(e) = self.cover_art_service.create_cover_art(cmd).await {
                    error!("Failed to create cover art, error:{}", e);
                    return;
                }
                return;
            }
            _ => return,
        }
    }
}
