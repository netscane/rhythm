use std::sync::Arc;

use super::shared::IdGenerator;
use crate::context::AppContext;
use crate::error::AppError;
use crate::event::event_bus::EventBus;
use crate::event::event_bus::EventEnvelope;
use domain::album::AlbumRepository;
use domain::annotation::Kind;
use domain::annotation::{Annotation, AnnotationRepository};
use domain::artist::ArtistRepository;
use domain::audio_file::{AudioFileError, AudioFileRepository};
use domain::event::DomainEvent;
use domain::player::{Player, PlayerRepository};
use domain::value::{AlbumId, AnnotationId, ArtistId, AudioFileId, PlayerId, UserId};

#[derive(Debug)]
pub struct StarItem {
    pub item_id: i64,
}

#[derive(Debug)]
pub struct StarCmd {
    pub user_id: UserId,
    pub items: Vec<StarItem>,
}

#[derive(Debug)]
pub struct UnstarItem {
    pub item_id: i64,
}

#[derive(Debug)]
pub struct UnstarCmd {
    pub user_id: UserId,
    pub items: Vec<UnstarItem>,
}

#[derive(Debug)]
pub struct SetRatingCmd {
    pub user_id: UserId,
    pub item_id: i64,
    pub rating: i32,
}

#[derive(Debug)]
pub struct ScrobbleCmd {
    pub player_id: PlayerId,
    /// client is the client name of the player, such as "web", "mobile", "desktop", etc.
    pub client: String,
    pub ip: String,
    pub user_agent: String,
    pub user_id: UserId,
    pub audio_file_id: AudioFileId,
    pub submission: bool,
}

pub struct MediaAnnotationService<B: EventBus> {
    media_annotation_repo: Arc<dyn AnnotationRepository>,
    audio_file_repository: Arc<dyn AudioFileRepository>,
    album_repository: Arc<dyn AlbumRepository>,
    artist_repository: Arc<dyn ArtistRepository>,
    player_repository: Arc<dyn PlayerRepository>,
    id_generator: Arc<dyn IdGenerator>,
    event_bus: Arc<B>,
}

impl<B: EventBus> MediaAnnotationService<B> {
    pub fn new(
        media_annotation_repo: Arc<dyn AnnotationRepository>,
        audio_file_repository: Arc<dyn AudioFileRepository>,
        album_repository: Arc<dyn AlbumRepository>,
        artist_repository: Arc<dyn ArtistRepository>,
        player_repository: Arc<dyn PlayerRepository>,
        id_generator: Arc<dyn IdGenerator>,
        event_bus: Arc<B>,
    ) -> Self {
        Self {
            media_annotation_repo,
            audio_file_repository,
            album_repository,
            artist_repository,
            player_repository,
            id_generator,
            event_bus,
        }
    }
    pub async fn scrobble(&self, ctx: &AppContext, cmd: ScrobbleCmd) -> Result<(), AppError> {
        if cmd.submission {
            self.scrobble_submission(ctx, cmd).await
        } else {
            self.scrobble_now_playing(ctx, cmd).await
        }
    }
    async fn scrobble_now_playing(
        &self,
        ctx: &AppContext,
        cmd: ScrobbleCmd,
    ) -> Result<(), AppError> {
        let mut player = match self.player_repository.find_by_id(cmd.player_id).await? {
            Some(player) => player,
            None => {
                let id = self.id_generator.next_id().await?;
                Player::new(
                    PlayerId::from(id),
                    cmd.user_id.clone(),
                    cmd.user_agent.clone(),
                    cmd.client.clone(),
                    cmd.ip.clone(),
                )
            }
        };
        player.play(cmd.audio_file_id.clone())?;
        let events = player.pop_events();
        self.player_repository.save(&mut player).await?;
        for event in events {
            let envelope = EventEnvelope::new(
                event.aggregate_id(),
                event.version(),
                event,
                ctx.correlation_id.clone(),
                ctx.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(())
    }

    async fn scrobble_submission(
        &self,
        ctx: &AppContext,
        cmd: ScrobbleCmd,
    ) -> Result<(), AppError> {
        let mut events = Vec::new();
        let audio_file = self
            .audio_file_repository
            .find_by_id(&cmd.audio_file_id)
            .await?
            .ok_or(AppError::AudioFileError(AudioFileError::NotFound(
                cmd.audio_file_id.clone(),
            )))?;

        match self
            .media_annotation_repo
            .find_by_item(Kind::AudioFile, cmd.audio_file_id.as_i64())
            .await?
        {
            Some(mut annotation) => {
                annotation.scrobble()?;
                events.extend(annotation.pop_events());
                self.media_annotation_repo.save(annotation).await?;
            }
            None => {
                let mut annot = Annotation::new(
                    AnnotationId::from(cmd.audio_file_id.as_i64()),
                    cmd.user_id.clone(),
                    Kind::AudioFile,
                    cmd.audio_file_id.as_i64(),
                );
                annot.scrobble()?;
                events.extend(annot.pop_events());
                self.media_annotation_repo.save(annot).await?;
            }
        };
        if let Some(album_id) = audio_file.album {
            match self
                .media_annotation_repo
                .find_by_item(Kind::Album, album_id.as_i64())
                .await?
            {
                Some(mut annotation) => {
                    annotation.scrobble()?;
                    events.extend(annotation.pop_events());
                    self.media_annotation_repo.save(annotation).await?;
                }
                None => {
                    let mut annotation = Annotation::new(
                        AnnotationId::from(album_id.as_i64()),
                        cmd.user_id.clone(),
                        Kind::Album,
                        album_id.as_i64(),
                    );
                    annotation.scrobble()?;
                    events.extend(annotation.pop_events());
                    self.media_annotation_repo.save(annotation).await?;
                }
            }
        }
        for participant in audio_file.participants {
            match self
                .media_annotation_repo
                .find_by_item(Kind::Artist, participant.artist_id.as_i64())
                .await?
            {
                Some(mut annotation) => {
                    annotation.scrobble()?;
                    events.extend(annotation.pop_events());
                    self.media_annotation_repo.save(annotation).await?;
                }
                None => {
                    let mut annotation = Annotation::new(
                        AnnotationId::from(participant.artist_id.as_i64()),
                        cmd.user_id.clone(),
                        Kind::Artist,
                        participant.artist_id.as_i64(),
                    );
                    annotation.scrobble()?;
                    events.extend(annotation.pop_events());
                    self.media_annotation_repo.save(annotation).await?;
                }
            }
        }
        let ctx = ctx.inherit();
        for event in events {
            let envelope = EventEnvelope::new(
                event.aggregate_id(),
                event.version(),
                event,
                ctx.correlation_id.clone(),
                ctx.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(())
    }

    /// 根据 ID 确定实体类型（AudioFile、Album 或 Artist）
    async fn determine_item_kind(&self, item_id: i64) -> Result<Option<Kind>, AppError> {
        // 先查询 AudioFile
        if self
            .audio_file_repository
            .find_by_id(&AudioFileId::from(item_id))
            .await?
            .is_some()
        {
            return Ok(Some(Kind::AudioFile));
        }

        // 再查询 Album
        if self
            .album_repository
            .by_id(AlbumId::from(item_id))
            .await?
            .is_some()
        {
            return Ok(Some(Kind::Album));
        }

        // 最后查询 Artist
        if self
            .artist_repository
            .by_id(ArtistId::from(item_id))
            .await?
            .is_some()
        {
            return Ok(Some(Kind::Artist));
        }

        Ok(None)
    }

    pub async fn star(&self, ctx: &AppContext, cmd: StarCmd) -> Result<(), AppError> {
        let mut all_events = Vec::new();
        for item in cmd.items {
            // 先查找已存在的 annotation
            let mut media_annotation = match self
                .media_annotation_repo
                .find_by_item_id(item.item_id)
                .await?
            {
                Some(annotation) => annotation,
                None => {
                    // 不存在时，需要确定 item 类型
                    let item_kind =
                        self.determine_item_kind(item.item_id)
                            .await?
                            .ok_or_else(|| {
                                AppError::AggregateNotFound(
                                    "Item".to_string(),
                                    format!(
                                        "id {} not found in audio_file, album or artist",
                                        item.item_id
                                    ),
                                )
                            })?;

                    let id = self.id_generator.next_id().await?;
                    Annotation::new(
                        AnnotationId::from(id),
                        cmd.user_id.clone(),
                        item_kind,
                        item.item_id,
                    )
                }
            };
            media_annotation.set_star()?;
            all_events.extend(media_annotation.pop_events());
            self.media_annotation_repo.save(media_annotation).await?;
        }
        let ctx = ctx.inherit();
        for event in all_events {
            let envelope = EventEnvelope::new(
                event.aggregate_id(),
                event.version(),
                event,
                ctx.correlation_id.clone(),
                ctx.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(())
    }

    pub async fn unstar(&self, ctx: &AppContext, cmd: UnstarCmd) -> Result<(), AppError> {
        let mut all_events = Vec::new();
        for item in cmd.items {
            let mut media_annotation = match self
                .media_annotation_repo
                .find_by_item_id(item.item_id)
                .await?
            {
                Some(annotation) => annotation,
                None => {
                    // unstar 时如果不存在，不需要创建新的 annotation
                    continue;
                }
            };
            media_annotation.unset_star()?;
            all_events.extend(media_annotation.pop_events());
            self.media_annotation_repo.save(media_annotation).await?;
        }
        let ctx = ctx.inherit();
        for event in all_events {
            let envelope = EventEnvelope::new(
                event.aggregate_id(),
                event.version(),
                event,
                ctx.correlation_id.clone(),
                ctx.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(())
    }

    pub async fn set_rating(&self, ctx: &AppContext, cmd: SetRatingCmd) -> Result<(), AppError> {
        let mut media_annotation = match self
            .media_annotation_repo
            .find_by_item_id(cmd.item_id)
            .await?
        {
            Some(annotation) => annotation,
            None => {
                // 不存在时，需要确定 item 类型
                let item_kind =
                    self.determine_item_kind(cmd.item_id)
                        .await?
                        .ok_or_else(|| {
                            AppError::AggregateNotFound(
                                "Item".to_string(),
                                format!(
                                    "id {} not found in audio_file, album or artist",
                                    cmd.item_id
                                ),
                            )
                        })?;

                let id = self.id_generator.next_id().await?;
                Annotation::new(
                    AnnotationId::from(id),
                    cmd.user_id.clone(),
                    item_kind,
                    cmd.item_id,
                )
            }
        };
        media_annotation.set_rating(cmd.rating)?;
        let events = media_annotation.pop_events();
        self.media_annotation_repo.save(media_annotation).await?;
        let ctx = ctx.inherit();
        for event in events {
            let envelope = EventEnvelope::new(
                event.aggregate_id(),
                event.version(),
                event,
                ctx.correlation_id.clone(),
                ctx.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(())
    }
}
