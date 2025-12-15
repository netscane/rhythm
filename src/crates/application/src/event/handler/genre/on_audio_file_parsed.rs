use crate::command::genre::{CreateGenreCmd, GenreService};
use crate::context::AppContext;
use crate::event::event_bus::{EventBus, EventEnvelope, Handler};
use crate::event::events::AppEvent;
use log::error;

#[derive(Clone)]
pub struct GenreOnAudioFileParsedHandler<B: EventBus> {
    genre_service: GenreService<B>,
}

impl<B: EventBus> GenreOnAudioFileParsedHandler<B> {
    pub fn new(genre_service: GenreService<B>) -> Self {
        Self { genre_service }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<AppEvent> for GenreOnAudioFileParsedHandler<B> {
    async fn handle(&self, envelope: &EventEnvelope<AppEvent>) {
        let evt = &envelope.payload;
        match evt {
            AppEvent::AudioFileParsed(evt) => {
                let ctx = AppContext::from(envelope);
                for genre in &evt.metadata.genres {
                    let cmd = CreateGenreCmd {
                        name: genre.to_string(),
                    };
                    if let Err(e) = self.genre_service.create_genre(&ctx, cmd).await {
                        error!("Failed to create genre, error:{}", e);
                    }
                }
            }
            _ => return,
        }
    }
}
