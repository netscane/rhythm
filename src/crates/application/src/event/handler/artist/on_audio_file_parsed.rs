use crate::command::artist::{ArtistService, CreateArtistCmd};
use crate::context::AppContext;
use crate::event::event_bus::{EventBus, EventEnvelope, Handler};
use crate::event::events::AppEvent;
use log::error;

#[derive(Clone)]
pub struct ArtistOnAudioFileParsedHandler<B: EventBus> {
    artist_service: ArtistService<B>,
}

impl<B: EventBus> ArtistOnAudioFileParsedHandler<B> {
    pub fn new(artist_service: ArtistService<B>) -> Self {
        Self { artist_service }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<AppEvent> for ArtistOnAudioFileParsedHandler<B> {
    async fn handle(&self, envelope: &EventEnvelope<AppEvent>) {
        let evt = &envelope.payload;
        match evt {
            AppEvent::AudioFileParsed(evt) => {
                let ctx = AppContext::from(envelope);
                for participant in &evt.metadata.participants {
                    let cmd = CreateArtistCmd {
                        name: participant.name.clone(),
                    };
                    if let Err(e) = self.artist_service.create_artist(&ctx, cmd).await {
                        error!("Failed to create artist, error:{}", e);
                    }
                }
            }
            _ => return,
        }
    }
}
