use crate::event::event_bus::{EventEnvelope, Handler};
use crate::projector::playback_history::PlaybackHistoryProjector;
use async_trait::async_trait;
use domain::annotation::AnnotationEvent;
use domain::value::AudioFileId;
use log::error;
use model::playback_history::PlaybackHistoryRepository;
use std::sync::Arc;

pub struct PlaybackHistoryEventHandler {
    projector: PlaybackHistoryProjector,
}

impl PlaybackHistoryEventHandler {
    pub fn new(repository: Arc<dyn PlaybackHistoryRepository>) -> Self {
        Self {
            projector: PlaybackHistoryProjector::new(repository),
        }
    }
}

#[async_trait]
impl Handler<AnnotationEvent> for PlaybackHistoryEventHandler {
    async fn handle(&self, envelope: &EventEnvelope<AnnotationEvent>) {
        match &envelope.payload {
            AnnotationEvent::ItemScrobbled {
                annotation_id: _,
                version: _,
                item_id,
                item_type,
                user_id,
            } => {
                if item_type != "audio_file" {
                    return;
                }
                if let Err(e) = self
                    .projector
                    .on_scrobble(AudioFileId::from(*item_id), user_id.clone())
                    .await
                {
                    error!("Error projecting player event: {}", e);
                }
            }
            _ => {}
        }
    }
}
