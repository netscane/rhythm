use crate::command::media_parse::{MediaFileParseService, ParseMediaFileCmd};
use crate::context::AppContext;
use crate::event::event_bus::{EventBus, EventEnvelope, Handler};
use domain::library::LibraryEvent;
use log::error;

#[derive(Clone)]
pub struct OnLibraryFileAddedHandler<B: EventBus> {
    media_file_parse_service: MediaFileParseService<B>,
}

impl<B: EventBus> OnLibraryFileAddedHandler<B> {
    pub fn new(media_file_parse_service: MediaFileParseService<B>) -> Self {
        Self {
            media_file_parse_service,
        }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<LibraryEvent> for OnLibraryFileAddedHandler<B> {
    async fn handle(&self, envelope: &EventEnvelope<LibraryEvent>) {
        let evt = &envelope.payload;
        match evt {
            LibraryEvent::FileAdded(evt) => {
                let ctx = AppContext::from(envelope);
                let cmd = ParseMediaFileCmd {
                    filemeta: evt.item.clone().into(),
                    library_id: evt.library_id.clone(),
                    file_type: evt.item.file_type.clone(),
                };
                if let Err(e) = self
                    .media_file_parse_service
                    .parse_media_file(&ctx, cmd)
                    .await
                {
                    error!("Failed to parse media file, error:{}", e);
                }
            }
            _ => return,
        }
    }
}
