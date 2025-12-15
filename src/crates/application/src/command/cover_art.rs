use crate::command::shared::IdGenerator;
use crate::context::AppContext;
use crate::error::AppError;
use crate::event::event_bus::{EventBus, EventEnvelope};
use domain::cover_art::{
    CoverArt, CoverArtDTO, CoverArtEvent, CoverArtRepository, CoverSourceType,
};
use domain::value::{AudioFileId, CoverArtId};
use domain::value::FileMeta;
use std::sync::Arc;

pub struct CreateCoverArtCmd {
    pub file_meta: FileMeta,
    pub source: CoverSourceType,
}

pub struct BindCmd {
    pub audio_file_id: AudioFileId,
    pub cover_art_id: CoverArtId,
}

#[derive(Clone)]
pub struct CoverArtService<B: EventBus> {
    pub cover_art_repository: Arc<dyn CoverArtRepository>,
    id_generator: Arc<dyn IdGenerator>,
    event_bus: Arc<B>,
}

impl<B: EventBus> CoverArtService<B> {
    pub fn new(
        cover_art_repository: Arc<dyn CoverArtRepository>,
        id_generator: Arc<dyn IdGenerator>,
        event_bus: Arc<B>,
    ) -> Self {
        Self {
            cover_art_repository,
            id_generator,
            event_bus,
        }
    }
}

impl<B: EventBus> CoverArtService<B> {
    pub async fn bind(&self, _context: &AppContext, cmd: BindCmd) -> Result<(), AppError> {
        let cover_art = self
            .cover_art_repository
            .find_by_id(&cmd.cover_art_id)
            .await?;
        if let Some(mut cover_art) = cover_art {
            cover_art.bind_to_audio_file(cmd.audio_file_id);
            self.cover_art_repository.save(cover_art).await?;
        }
        Ok(())
    }
    pub async fn create_cover_art(
        &self,
        context: &AppContext,
        cmd: CreateCoverArtCmd,
    ) -> Result<(), AppError> {
        let new_id = self.id_generator.next_id().await?;

        let dto = CoverArtDTO {
            audio_file_id: None,
            album_id: None,
            path: cmd.file_meta.path.clone(),
            width: None,
            height: None,
            format: None,
            file_size: cmd.file_meta.size,
            source: cmd.source,
        };
        let mut co_art = CoverArt::from_dto(new_id.into(), dto)?;
        co_art = self.cover_art_repository.save(co_art).await?;
        let evts = co_art.take_events();
        for evt in evts {
            let envelope = EventEnvelope::<CoverArtEvent>::new_with_domain_event(
                evt,
                context.correlation_id.clone(),
                context.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(())
    }
}
