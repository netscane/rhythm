use crate::command::audio_file::{AudioFileService, CreateAudioFileCmd};
use crate::context::AppContext;
use crate::event::event_bus::{EventBus, EventEnvelope, Handler};
use crate::event::events::AppEvent;
use log::error;

#[derive(Clone)]
pub struct AudioFileOnAudioFileParsedHandler<B: EventBus> {
    audio_file_service: AudioFileService<B>,
}

impl<B: EventBus> AudioFileOnAudioFileParsedHandler<B> {
    pub fn new(audio_file_service: AudioFileService<B>) -> Self {
        Self { audio_file_service }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<AppEvent> for AudioFileOnAudioFileParsedHandler<B> {
    async fn handle(&self, envelope: &EventEnvelope<AppEvent>) {
        let evt = &envelope.payload;
        match evt {
            AppEvent::AudioFileParsed(evt) => {
                let ctx = AppContext::from(envelope);
                let cmd = CreateAudioFileCmd {
                    filemeta: evt.file_info.clone(),
                    audio_metadata: evt.metadata.clone(),
                    library_id: evt.library_id.clone(),
                };
                if let Err(e) = self.audio_file_service.create_audio_file(&ctx, cmd).await {
                    error!("Failed to create audio file, error:{}", e);
                }
            }
            _ => return,
        }
    }
}
