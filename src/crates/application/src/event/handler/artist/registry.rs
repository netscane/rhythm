use super::on_audio_file_parsed::ArtistOnAudioFileParsedHandler;
use crate::command::artist::ArtistService;
use crate::event::event_bus::EventBus;
use crate::event::events::AppEvent;
use std::sync::Arc;

pub async fn register_handlers<B: EventBus + Clone + 'static>(
    bus: &mut B,
    artist_service: ArtistService<B>,
) {
    let handler = ArtistOnAudioFileParsedHandler::new(artist_service);
    bus.subscribe::<AppEvent>(Arc::new(handler)).await;
}
