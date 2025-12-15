use super::on_image_file_parsed::CoverArtOnImageFileParsedHandler;
use crate::command::cover_art::CoverArtService;
use crate::event::event_bus::EventBus;
use crate::event::events::AppEvent;
use std::sync::Arc;

pub async fn register_handlers<B: EventBus + Clone + 'static>(
    bus: &mut B,
    cover_art_service: CoverArtService<B>,
) {
    let handler = CoverArtOnImageFileParsedHandler::new(cover_art_service);
    bus.subscribe::<AppEvent>(Arc::new(handler)).await;
}
