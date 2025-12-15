use crate::event::event_bus::{EventEnvelope, Handler};
use crate::projector::artist_location::ArtistLocationProjector;
use domain::audio_file::{AudioFileEvent, AudioFileEventKind};
use log::{debug, error};

pub struct ArtistLocationHandler {
    projector: ArtistLocationProjector,
}

impl ArtistLocationHandler {
    pub fn new(projector: ArtistLocationProjector) -> Self {
        Self { projector }
    }
}

#[async_trait::async_trait]
impl Handler<AudioFileEvent> for ArtistLocationHandler {
    async fn handle(&self, event: &EventEnvelope<AudioFileEvent>) {
        match &event.payload.kind {
            AudioFileEventKind::ParticipantAdded(_) => {
                if let Err(e) = self
                    .projector
                    .on_audio_file_participant_added(&event.payload)
                    .await
                {
                    error!("Failed to handle audio file participant added event: {}", e);
                }
            }
            AudioFileEventKind::ParticipantRemoved(_) => {
                if let Err(e) = self
                    .projector
                    .on_audio_file_participant_removed(&event.payload)
                    .await
                {
                    error!("Failed to handle audio file participant removed event: {}", e);
                }
            }
            _ => {
                debug!("Audio file event received, no action needed for artist location");
            }
        }
    }
}
