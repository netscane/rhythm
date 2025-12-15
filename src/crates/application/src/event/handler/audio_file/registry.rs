use super::on_audio_file_parsed::AudioFileOnAudioFileParsedHandler;
use crate::command::audio_file::AudioFileService;
use crate::event::event_bus::EventBus;
use crate::event::events::AppEvent;
use std::sync::Arc;

pub async fn register_handlers<B: EventBus + Clone + 'static>(
    bus: &mut B,
    audio_file_service: AudioFileService<B>,
) {
    let handler = AudioFileOnAudioFileParsedHandler::new(audio_file_service);
    bus.subscribe::<AppEvent>(Arc::new(handler)).await;
}
