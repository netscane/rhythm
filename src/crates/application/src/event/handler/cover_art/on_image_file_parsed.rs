use crate::command::cover_art::{CoverArtService, CreateCoverArtCmd};
use crate::context::AppContext;
use crate::event::event_bus::{EventBus, EventEnvelope, Handler};
use crate::event::events::AppEvent;
use log::error;

#[derive(Clone)]
pub struct CoverArtOnImageFileParsedHandler<B: EventBus> {
    cover_art_service: CoverArtService<B>,
}

impl<B: EventBus> CoverArtOnImageFileParsedHandler<B> {
    pub fn new(cover_art_service: CoverArtService<B>) -> Self {
        Self { cover_art_service }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<AppEvent> for CoverArtOnImageFileParsedHandler<B> {
    async fn handle(&self, envelope: &EventEnvelope<AppEvent>) {
        let evt = &envelope.payload;
        match evt {
            AppEvent::ImageFileParsed(evt) => {
                let ctx = AppContext::from(envelope);
                let cmd = CreateCoverArtCmd {
                    file_meta: evt.file_info.clone(),
                    source: evt.source.clone(),
                };
                if let Err(e) = self.cover_art_service.create_cover_art(&ctx, cmd).await {
                    error!("Failed to create cover art, error:{}", e);
                }
            }
            _ => return,
        }
    }
}
