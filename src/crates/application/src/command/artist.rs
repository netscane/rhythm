use crate::command::shared::IdGenerator;
use crate::context::AppContext;
use crate::error::AppError;
use crate::event::event_bus::{EventBus, EventEnvelope};
use domain::artist::{Artist, ArtistEvent, ArtistFound, ArtistRepository};
use domain::value::{ArtistId, GenreId};
use log::info;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub struct CreateArtistCmd {
    pub name: String,
}

#[derive(Debug)]
pub struct BindCmd {
    pub artist_id: ArtistId,
    pub genre_ids: Vec<GenreId>,
}

pub trait ArtistNameNormalizer: Send + Sync {
    fn normalize(&self, artist_name: &String) -> String;
}

#[derive(Clone)]
pub struct ArtistService<B: EventBus> {
    id_generator: Arc<dyn IdGenerator>,
    artist_repository: Arc<dyn ArtistRepository>,
    artist_name_normalizer: Arc<dyn ArtistNameNormalizer>,
    event_bus: Arc<B>,
}

impl<B: EventBus> ArtistService<B> {
    pub fn new(
        id_generator: Arc<dyn IdGenerator>,
        artist_repository: Arc<dyn ArtistRepository>,
        artist_name_normalizer: Arc<dyn ArtistNameNormalizer>,
        event_bus: Arc<B>,
    ) -> Self {
        Self {
            id_generator,
            artist_repository,
            artist_name_normalizer,
            event_bus,
        }
    }

    pub async fn create_artist(
        &self,
        context: &AppContext,
        cmd: CreateArtistCmd,
    ) -> Result<Artist, AppError> {
        let sort_name = self.artist_name_normalizer.normalize(&cmd.name);
        let artist = self.artist_repository.find_by_sort_name(&sort_name).await?;
        if let Some(artist) = artist {
            let event = ArtistEvent::Found(ArtistFound {
                artist_id: artist.id.clone(),
                version: artist.version,
                name: artist.name.clone(),
                sort_name: artist.sort_name.clone(),
            });

            let envelope = EventEnvelope::new(
                artist.id.as_i64(),
                artist.version,
                event,
                context.correlation_id.clone(),
                context.causation_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
            return Ok(artist);
        }
        let artist_id = self.id_generator.next_id().await?;
        let mut artist = Artist::new(artist_id.into(), cmd.name, sort_name);
        let events = artist.take_events();
        let artist = self.artist_repository.save(artist).await?;
        for event in events {
            let envelope = EventEnvelope::new(
                artist.id.as_i64(),
                artist.version,
                event,
                context.correlation_id.clone(),
                context.causation_id.clone(),
            );
            self.event_bus.publish(envelope).await?
        }
        Ok(artist)
    }

    pub async fn bind(&self, context: &AppContext, cmd: BindCmd) -> Result<(), AppError> {
        let mut artist = self
            .artist_repository
            .by_id(cmd.artist_id.clone())
            .await?
            .ok_or_else(|| {
                AppError::AggregateNotFound("Artist".to_string(), cmd.artist_id.to_string())
            })?;
        for genre_id in cmd.genre_ids {
            artist.bind_to_genre(genre_id)?;
        }
        let events = artist.take_events();
        let artist = self.artist_repository.save(artist).await?;
        for event in events {
            let envelope = EventEnvelope::new(
                artist.id.as_i64(),
                artist.version,
                event,
                context.correlation_id.clone(),
                context.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(())
    }
}
