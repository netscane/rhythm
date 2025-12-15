use crate::command::shared::IdGenerator;
use crate::context::AppContext;
use crate::error::AppError;
use crate::event::event_bus::{EventBus, EventEnvelope};
use domain::album::{Album, AlbumEvent, AlbumEventKind, AlbumFound, AlbumRepository};
use domain::value::{AlbumId, ArtistId, GenreId, ParticipantRole, ParticipantSubRole};
use std::sync::Arc;

#[derive(Debug)]
pub struct CreateAlbumCmd {
    pub name: String,
}

#[derive(Debug)]
pub struct BindCmd {
    pub album_id: AlbumId,
    pub genre_ids: Vec<GenreId>,
    pub artists: Vec<(ArtistId, ParticipantRole, Option<ParticipantSubRole>)>,
}

pub trait AlbumNameNormalizer: Send + Sync {
    fn normalize(&self, album_name: &String) -> String;
}

#[derive(Clone)]
pub struct AlbumService<B: EventBus> {
    id_generator: Arc<dyn IdGenerator>,
    album_repository: Arc<dyn AlbumRepository>,
    album_name_normalizer: Arc<dyn AlbumNameNormalizer>,
    event_bus: Arc<B>,
}

impl<B: EventBus> AlbumService<B> {
    pub fn new(
        id_generator: Arc<dyn IdGenerator>,
        album_repository: Arc<dyn AlbumRepository>,
        album_name_normalizer: Arc<dyn AlbumNameNormalizer>,
        event_bus: Arc<B>,
    ) -> Self {
        Self {
            id_generator,
            album_repository,
            album_name_normalizer,
            event_bus,
        }
    }

    pub async fn create_album(
        &self,
        context: &AppContext,
        cmd: CreateAlbumCmd,
    ) -> Result<Album, AppError> {
        let sort_name = self.album_name_normalizer.normalize(&cmd.name);
        let album = self.album_repository.find_by_sort_name(&sort_name).await?;
        if let Some(album) = album {
            let event_kind = AlbumEventKind::Found(AlbumFound {
                album_id: album.id.clone(),
                name: album.name.clone(),
                sort_name: album.sort_name.clone(),
                genres: album.genres.iter().map(|id| id.to_string()).collect(),
            });
            let event = AlbumEvent {
                album_id: album.id.clone(),
                version: album.version,
                kind: event_kind,
            };
            let envelope = EventEnvelope::new(
                album.id.as_i64(),
                album.version,
                event,
                context.correlation_id.clone(),
                context.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
            return Ok(album);
        }
        let album_id = self.id_generator.next_id().await?;
        let mut album = Album::new(album_id.into(), cmd.name, sort_name);
        let events = album.take_events();
        let album = self.album_repository.save(album).await?;
        for event in events {
            let envelope = EventEnvelope::new(
                album.id.as_i64(),
                album.version,
                event,
                context.correlation_id.clone(),
                context.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(album)
    }

    pub async fn bind(&self, context: &AppContext, cmd: BindCmd) -> Result<(), AppError> {
        let mut album = self
            .album_repository
            .by_id(cmd.album_id.clone())
            .await?
            .ok_or_else(|| {
                AppError::AggregateNotFound("Album".to_string(), cmd.album_id.to_string())
            })?;

        // 绑定流派
        for genre_id in cmd.genre_ids {
            album.bind_to_genre(genre_id)?;
        }

        // 绑定艺术家
        for (artist_id, role, sub_role) in cmd.artists {
            let participant = domain::value::Participant {
                artist_id,
                role,
                sub_role,
                work_id: cmd.album_id.as_i64(),
                work_type: domain::value::ParticipantWorkType::Album,
            };
            album.add_participant(participant)?;
        }

        // 保存并发布事件
        let events = album.take_events();
        let album = self.album_repository.save(album).await?;

        for event in events {
            let envelope = EventEnvelope::new(
                album.id.as_i64(),
                album.version,
                event,
                context.correlation_id.clone(),
                context.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(())
    }
}
