use super::on_audio_file_parsed::AlbumOnAudioFileParsedHandler;
use crate::command::album::AlbumService;
use crate::event::event_bus::EventBus;
use crate::event::events::AppEvent;
use std::sync::Arc;

pub async fn register_handlers<B: EventBus + Clone + 'static>(
    bus: &mut B,
    album_service: AlbumService<B>,
) {
    let handler = AlbumOnAudioFileParsedHandler::new(album_service);
    bus.subscribe::<AppEvent>(Arc::new(handler)).await;
}
