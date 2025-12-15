use std::sync::Arc;

use crate::command::artist::{ArtistService, BindCmd};
use crate::context::AppContext;
use crate::event::event_bus::{CorrelationId, EventBus, EventEnvelope, EventId, Handler};
use crate::event::events::AppEvent;
use domain::artist::ArtistEvent;
use domain::genre::GenreEvent;
use domain::value::{ArtistId, GenreId};
use log::{error, info};
use std::collections::HashMap;
use tokio::sync::Mutex;
#[derive(Clone)]
pub struct BindToArtistCoordinator<B: EventBus> {
    artist_service: ArtistService<B>,
    // caches to correlate events by media path
    pending_genres_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, Vec<GenreId>>>>,
    pending_artists_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, Vec<ArtistId>>>>,
    pending_audio_genres_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, Vec<String>>>>,
    pending_audio_artists_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, Vec<String>>>>,
}

impl<B: EventBus> BindToArtistCoordinator<B> {
    pub fn new(artist_service: ArtistService<B>) -> Self {
        Self {
            artist_service,
            pending_genres_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
            pending_artists_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
            pending_audio_genres_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
            pending_audio_artists_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
        }
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
    async fn on_audio_file_parsed(
        &self,
        ctx: &AppContext,
        evt: &crate::event::events::AudioFileParsed,
    ) {
        {
            let mut audio_genres_cache = self.pending_audio_genres_by_correlation_id.lock().await;
            let mut audio_artists_cache = self.pending_audio_artists_by_correlation_id.lock().await;
            // 从AudioFileParsed的metadata中提取流派和艺术家信息
            audio_genres_cache.insert(ctx.correlation_id.clone(), evt.metadata.genres.clone());
            audio_artists_cache.insert(
                ctx.correlation_id.clone(),
                evt.metadata
                    .participants
                    .clone()
                    .into_iter()
                    .map(|p| p.name)
                    .collect(),
            );
        } // 释放锁
          // 检查是否可以执行绑定操作
        self.check_and_bind(&ctx).await;
    }

    pub async fn check_and_bind(&self, ctx: &AppContext) {
        // 按固定顺序获取锁并检查数据: artist -> genre -> audio_artists -> audio_genres
        let artist_ids = {
            let artist_cache = self.pending_artists_by_correlation_id.lock().await;
            artist_cache.get(&ctx.correlation_id).cloned()
        };
        
        if artist_ids.is_none() {
            return;
        }
        
        let genre_ids = {
            let genre_cache = self.pending_genres_by_correlation_id.lock().await;
            genre_cache.get(&ctx.correlation_id).cloned()
        };
        
        if genre_ids.is_none() {
            return;
        }
        
        let audio_artists = {
            let audio_artists_cache = self.pending_audio_artists_by_correlation_id.lock().await;
            audio_artists_cache.get(&ctx.correlation_id).cloned()
        };
        
        if audio_artists.is_none() {
            return;
        }
        
        let audio_genres = {
            let audio_genres_cache = self.pending_audio_genres_by_correlation_id.lock().await;
            audio_genres_cache.get(&ctx.correlation_id).cloned()
        };
        
        if audio_genres.is_none() {
            return;
        }

        let artist_ids = artist_ids.unwrap();
        let genre_ids = genre_ids.unwrap();
        let audio_genres = audio_genres.unwrap();
        let audio_artists = audio_artists.unwrap();

        if audio_genres.len() != genre_ids.len() || audio_artists.len() != artist_ids.len() {
            return;
        }

        // 在执行绑定操作前，所有锁都已释放
        self.execute_batch_binding(ctx, artist_ids, genre_ids).await;
    }

    async fn execute_batch_binding(
        &self,
        ctx: &AppContext,
        artist_ids: Vec<ArtistId>,
        genre_ids: Vec<GenreId>,
    ) {
        for artist_id in &artist_ids {
            let cmd = BindCmd {
                genre_ids: genre_ids.clone(),
                artist_id: artist_id.clone(),
            };
            let ctx = AppContext {
                event_id: EventId::new(),
                correlation_id: ctx.correlation_id.clone(),
                causation_id: ctx.event_id.clone(),
            };

            if let Err(e) = self.artist_service.bind(&ctx, cmd).await {
                error!(
                    "failed to bind artist: {} to genres: {:?} for correlation_id: {:?}, error: {}",
                    artist_id, genre_ids, ctx.correlation_id, e
                );
            }
        }
    }
}
#[async_trait::async_trait]
impl<B: EventBus> Handler<ArtistEvent> for BindToArtistCoordinator<B> {
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
impl<B: EventBus> Handler<GenreEvent> for BindToArtistCoordinator<B> {
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
impl<B: EventBus> Handler<AppEvent> for BindToArtistCoordinator<B> {
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
