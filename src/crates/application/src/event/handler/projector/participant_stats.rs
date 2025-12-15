use crate::event::event_bus::{EventEnvelope, Handler};
use crate::projector::participant_stats::ParticipantStatsProjector;
use domain::album::{AlbumEvent, AlbumEventKind};
use domain::audio_file::{AudioFileEvent, AudioFileEventKind};
use log::{debug, error};

pub struct ParticipantStatsHandler {
    participant_stats_projector: ParticipantStatsProjector,
}

impl ParticipantStatsHandler {
    pub fn new(participant_stats_projector: ParticipantStatsProjector) -> Self {
        Self {
            participant_stats_projector,
        }
    }
}

#[async_trait::async_trait]
impl Handler<AudioFileEvent> for ParticipantStatsHandler {
    async fn handle(&self, event_envelope: &EventEnvelope<AudioFileEvent>) {
        match &event_envelope.payload.kind {
            AudioFileEventKind::ParticipantAdded(_) => {
                if let Err(e) = self
                    .participant_stats_projector
                    .on_audio_file_participant_added(&event_envelope.payload)
                    .await
                {
                    error!("Failed to handle audio file participant added event: {}", e);
                }
            }
            AudioFileEventKind::ParticipantRemoved(_) => {
                if let Err(e) = self
                    .participant_stats_projector
                    .on_audio_file_participant_removed(&event_envelope.payload)
                    .await
                {
                    error!(
                        "Failed to handle audio file participant removed event: {}",
                        e
                    );
                }
            }
            _ => {
                // 其他事件不需要处理
                debug!("Audio file event received, no action needed for artist stat");
            }
        }
    }
}

#[async_trait::async_trait]
impl Handler<AlbumEvent> for ParticipantStatsHandler {
    async fn handle(&self, event_envelope: &EventEnvelope<AlbumEvent>) {
        match &event_envelope.payload.kind {
            AlbumEventKind::ParticipantAdded(_) => {
                if let Err(e) = self
                    .participant_stats_projector
                    .on_album_participant_added(&event_envelope.payload)
                    .await
                {
                    error!("Failed to handle album participant added event: {}", e);
                }
            }
            AlbumEventKind::ParticipantRemoved(_) => {
                if let Err(e) = self
                    .participant_stats_projector
                    .on_album_participant_removed(&event_envelope.payload)
                    .await
                {
                    error!("Failed to handle album participant removed event: {}", e);
                }
            }
            _ => {
                // 其他事件不需要处理
                debug!("Album event received, no action needed for participant stats");
            }
        }
    }
}
