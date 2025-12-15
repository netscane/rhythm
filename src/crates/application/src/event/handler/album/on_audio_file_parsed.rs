use crate::command::album::{AlbumService, CreateAlbumCmd};
use crate::context::AppContext;
use crate::event::event_bus::{EventBus, EventEnvelope, Handler};
use crate::event::events::AppEvent;
use log::error;

#[derive(Clone)]
pub struct AlbumOnAudioFileParsedHandler<B: EventBus> {
    album_service: AlbumService<B>,
}

impl<B: EventBus> AlbumOnAudioFileParsedHandler<B> {
    pub fn new(album_service: AlbumService<B>) -> Self {
        Self { album_service }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<AppEvent> for AlbumOnAudioFileParsedHandler<B> {
    async fn handle(&self, envelope: &EventEnvelope<AppEvent>) {
        let evt = &envelope.payload;
        match evt {
            AppEvent::AudioFileParsed(evt) => {
                let ctx = AppContext::from(envelope);
                let cmd = CreateAlbumCmd {
                    name: evt.metadata.album.clone(),
                };
                if let Err(e) = self.album_service.create_album(&ctx, cmd).await {
                    error!("Failed to create album, error:{}", e);
                }
            }
            _ => return,
        }
    }
}
