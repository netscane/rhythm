use super::on_audio_file_parsed::GenreOnAudioFileParsedHandler;
use crate::command::genre::GenreService;
use crate::event::event_bus::EventBus;
use crate::event::events::AppEvent;
use std::sync::Arc;

pub async fn register_handlers<B: EventBus + Clone + 'static>(
    bus: &mut B,
    genre_service: GenreService<B>,
) {
    let handler = GenreOnAudioFileParsedHandler::new(genre_service);
    bus.subscribe::<AppEvent>(Arc::new(handler)).await;
}
