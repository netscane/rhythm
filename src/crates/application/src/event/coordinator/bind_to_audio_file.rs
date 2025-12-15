use std::sync::Arc;

use crate::command::audio_file::{AudioFileService, BindCmd};
use crate::context::AppContext;
use crate::event::event_bus::{CorrelationId, EventBus, EventEnvelope, Handler};
use crate::event::events::AppEvent;
use domain::album::{AlbumEvent, AlbumEventKind};
use domain::artist::ArtistEvent;
use domain::audio_file::AudioFileEvent;
use domain::genre::GenreEvent;
use domain::value::{AlbumId, ArtistId, AudioFileId, GenreId, ParticipantRole, ParticipantSubRole};
use log::error;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct BindToAudioFileCoordinator<B: EventBus> {
    audio_file_service: AudioFileService<B>,
    // caches to correlate events by media path
    pending_artists_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, Vec<ArtistId>>>>,
    pending_genres_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, Vec<GenreId>>>>,
    pending_album_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, AlbumId>>>,
    pending_audio_file_id_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, AudioFileId>>>,
    pending_audio_artists_by_correlation_id: Arc<
        Mutex<HashMap<CorrelationId, Vec<(String, ParticipantRole, Option<ParticipantSubRole>)>>>,
    >,
    pending_audio_genres_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, Vec<String>>>>,
}

impl<B: EventBus> BindToAudioFileCoordinator<B> {
    pub fn new(audio_file_service: AudioFileService<B>) -> Self {
        Self {
            audio_file_service,
            pending_artists_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
            pending_genres_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
            pending_album_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
            pending_audio_file_id_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
            pending_audio_artists_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
            pending_audio_genres_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn on_album_available(&self, ctx: &AppContext, album_id: &AlbumId) {
        {
            let mut album_cache = self.pending_album_by_correlation_id.lock().await;
            album_cache.insert(ctx.correlation_id.clone(), album_id.clone());
        } // 释放锁
          // 检查是否可以执行绑定操作
        self.check_and_bind(&ctx).await;
    }

    async fn on_audio_file_available(&self, ctx: &AppContext, audio_file_id: &AudioFileId) {
        {
            let mut audio_file_cache = self.pending_audio_file_id_by_correlation_id.lock().await;
            audio_file_cache.insert(ctx.correlation_id.clone(), audio_file_id.clone());
        } // 释放锁
          // 检查是否可以执行绑定操作
        self.check_and_bind(&ctx).await;
    }

    async fn on_artist_available(&self, ctx: &AppContext, artist_id: &ArtistId) {
        {
            let mut artist_cache = self.pending_artists_by_correlation_id.lock().await;
            let artists = artist_cache
                .entry(ctx.correlation_id.clone())
                .or_insert_with(Vec::new);
            artists.push(artist_id.clone());
        } // 释放锁
          // 检查是否可以执行绑定操作
        self.check_and_bind(&ctx).await;
    }

    async fn on_genre_available(&self, ctx: &AppContext, genre_id: &GenreId) {
        {
            let mut genres_cache = self.pending_genres_by_correlation_id.lock().await;
            genres_cache
                .entry(ctx.correlation_id.clone())
                .or_insert_with(Vec::new)
                .push(genre_id.clone());
        } // 释放锁
          // 检查是否可以执行绑定操作
        self.check_and_bind(&ctx).await;
    }

    async fn on_audio_file_parsed(
        &self,
        ctx: &AppContext,
        evt: &crate::event::events::AudioFileParsed,
    ) {
        {
            let mut audio_artists_cache = self.pending_audio_artists_by_correlation_id.lock().await;
            let mut audio_genres_cache = self.pending_audio_genres_by_correlation_id.lock().await;

            // 从AudioFileParsed的metadata中提取艺术家信息
            let artists_with_roles: Vec<(String, ParticipantRole, Option<ParticipantSubRole>)> =
                evt.metadata
                    .participants
                    .iter()
                    .map(|p| (p.name.clone(), p.role.clone(), p.sub_role.clone()))
                    .collect();
            audio_artists_cache.insert(ctx.correlation_id.clone(), artists_with_roles);

            // 从AudioFileParsed的metadata中提取流派信息
            audio_genres_cache.insert(ctx.correlation_id.clone(), evt.metadata.genres.clone());
        } // 释放锁
          // 检查是否可以执行绑定操作
        self.check_and_bind(&ctx).await;
    }

    pub async fn check_and_bind(&self, ctx: &AppContext) {
        // 检查是否有音频文件
        let audio_file_id = {
            let audio_file_cache = self.pending_audio_file_id_by_correlation_id.lock().await;
            audio_file_cache.get(&ctx.correlation_id).cloned()
        };

        if let Some(audio_file_id) = audio_file_id {
            // 检查是否所有必要的数据都准备好了
            // 按固定顺序获取锁: album -> artist -> genre -> audio_artists -> audio_genres
            let album_id = {
                let album_cache = self.pending_album_by_correlation_id.lock().await;
                album_cache.get(&ctx.correlation_id).cloned()
            };
            
            let artists = {
                let artist_cache = self.pending_artists_by_correlation_id.lock().await;
                artist_cache.get(&ctx.correlation_id).cloned()
            };
            
            let genre_ids = {
                let genre_cache = self.pending_genres_by_correlation_id.lock().await;
                genre_cache.get(&ctx.correlation_id).cloned()
            };
            
            let audio_artists = {
                let audio_artists_cache = self.pending_audio_artists_by_correlation_id.lock().await;
                audio_artists_cache.get(&ctx.correlation_id).cloned()
            };
            
            let audio_genres = {
                let audio_genres_cache = self.pending_audio_genres_by_correlation_id.lock().await;
                audio_genres_cache.get(&ctx.correlation_id).cloned()
            };

            // 检查是否所有数据都准备好了
            let (genre_ids, artists) = if let (
                Some(genre_ids),
                Some(artists),
                Some(_),
                Some(audio_genres),
                Some(audio_artists),
            ) = (genre_ids, artists, album_id, audio_genres, audio_artists)
            {
                if genre_ids.len() == audio_genres.len() && artists.len() == audio_artists.len()
                {
                    (Some(genre_ids), Some((artists, audio_artists)))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            if let (Some(genre_ids), Some((artists, audio_artists))) = (genre_ids, artists) {
                // 获取 album_id
                let album_id = {
                    let album_cache = self.pending_album_by_correlation_id.lock().await;
                    album_cache.get(&ctx.correlation_id).cloned()
                };

                if let Some(album_id) = album_id {
                    // 清理缓存
                    self.cleanup_caches(&ctx).await;

                    // 执行批量绑定
                    self.execute_batch_binding(
                        &ctx,
                        &audio_file_id,
                        &album_id,
                        genre_ids,
                        artists,
                        audio_artists,
                    )
                    .await;
                }
            }
        }
    }

    async fn cleanup_caches(&self, ctx: &AppContext) {
        // 按固定顺序获取锁,避免死锁
        // 顺序: audio_file -> album -> artist -> genre -> audio_artists -> audio_genres
        {
            let mut audio_file_cache = self.pending_audio_file_id_by_correlation_id.lock().await;
            audio_file_cache.remove(&ctx.correlation_id);
        }
        {
            let mut album_cache = self.pending_album_by_correlation_id.lock().await;
            album_cache.remove(&ctx.correlation_id);
        }
        {
            let mut artist_cache = self.pending_artists_by_correlation_id.lock().await;
            artist_cache.remove(&ctx.correlation_id);
        }
        {
            let mut genre_cache = self.pending_genres_by_correlation_id.lock().await;
            genre_cache.remove(&ctx.correlation_id);
        }
        {
            let mut audio_artists_cache = self.pending_audio_artists_by_correlation_id.lock().await;
            audio_artists_cache.remove(&ctx.correlation_id);
        }
        {
            let mut audio_genres_cache = self.pending_audio_genres_by_correlation_id.lock().await;
            audio_genres_cache.remove(&ctx.correlation_id);
        }
    }

    async fn execute_batch_binding(
        &self,
        ctx: &AppContext,
        audio_file_id: &AudioFileId,
        album_id: &AlbumId,
        genre_ids: Vec<GenreId>,
        artists: Vec<ArtistId>,
        audio_artists: Vec<(String, ParticipantRole, Option<ParticipantSubRole>)>,
    ) {
        // 准备艺术家数据
        let artists_with_roles: Vec<(ArtistId, ParticipantRole, Option<ParticipantSubRole>)> =
            artists
                .iter()
                .zip(audio_artists.iter())
                .map(|(artist_id, (_, role, sub_role))| {
                    (artist_id.clone(), role.clone(), sub_role.clone())
                })
                .collect();

        // 创建批量绑定命令
        let cmd = BindCmd {
            audio_file_id: audio_file_id.clone(),
            album_id: album_id.clone(),
            genre_ids,
            artists: artists_with_roles,
        };

        let ctx = ctx.inherit();

        if let Err(e) = self.audio_file_service.bind(&ctx, cmd).await {
            error!("Failed to bind: {}", e);
        }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<ArtistEvent> for BindToAudioFileCoordinator<B> {
    async fn handle(&self, event: &EventEnvelope<ArtistEvent>) {
        let ctx = AppContext::from(event);
        match &event.payload {
            ArtistEvent::Found(found) => {
                self.on_artist_available(&ctx, &found.artist_id).await;
            }
            ArtistEvent::Created(created) => {
                self.on_artist_available(&ctx, &created.artist_id).await;
            }
            _ => {}
        }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<GenreEvent> for BindToAudioFileCoordinator<B> {
    async fn handle(&self, event: &EventEnvelope<GenreEvent>) {
        let ctx = AppContext::from(event);
        match &event.payload {
            GenreEvent::Created(created) => {
                self.on_genre_available(&ctx, &created.genre_id).await;
            }
            GenreEvent::Found(found) => {
                self.on_genre_available(&ctx, &found.genre_id).await;
            }
        }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<AlbumEvent> for BindToAudioFileCoordinator<B> {
    async fn handle(&self, event: &EventEnvelope<AlbumEvent>) {
        let ctx = AppContext::from(event);
        match &event.payload.kind {
            AlbumEventKind::Created(_) => {
                self.on_album_available(&ctx, &event.payload.album_id).await;
            }
            AlbumEventKind::Found(_) => {
                self.on_album_available(&ctx, &event.payload.album_id).await;
            }
            _ => {}
        }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<AudioFileEvent> for BindToAudioFileCoordinator<B> {
    async fn handle(&self, event: &EventEnvelope<AudioFileEvent>) {
        let ctx = AppContext::from(event);
        match &event.payload.kind {
            domain::audio_file::AudioFileEventKind::Created(created) => {
                self.on_audio_file_available(&ctx, &created.audio_file_id)
                    .await;
            }
            _ => {}
        }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<AppEvent> for BindToAudioFileCoordinator<B> {
    async fn handle(&self, event: &EventEnvelope<AppEvent>) {
        let ctx = AppContext::from(event);
        match &event.payload {
            AppEvent::AudioFileParsed(audio_file_parsed) => {
                self.on_audio_file_parsed(&ctx, audio_file_parsed).await;
            }
            _ => {}
        }
    }
}
