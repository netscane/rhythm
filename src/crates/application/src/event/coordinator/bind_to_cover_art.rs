use std::sync::Arc;

use crate::command::cover_art::{BindCmd, CoverArtService};
use crate::context::AppContext;
use crate::event::event_bus::{CorrelationId, EventBus, EventEnvelope, Handler};

use domain::audio_file::{AudioFileEvent, AudioFileEventKind};
use domain::cover_art::{CoverArtEvent, CoverArtEventKind, CoverSourceType};
use domain::value::{AudioFileId, CoverArtId};

use log::{error, info};
use std::collections::HashMap;
use tokio::sync::Mutex;
#[derive(Clone)]
pub struct BindToCoverArtCoordinator<B: EventBus + Send + Sync> {
    cover_art_service: CoverArtService<B>,
    // caches to correlate events by media path
    pending_audio_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, AudioFileId>>>,
    pending_cover_by_correlation_id: Arc<Mutex<HashMap<CorrelationId, CoverArtId>>>,
}

impl<B: EventBus + Send + Sync> BindToCoverArtCoordinator<B> {
    pub fn new(cover_art_service: CoverArtService<B>) -> Self {
        Self {
            cover_art_service,
            pending_audio_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
            pending_cover_by_correlation_id: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub async fn check_and_bind(&self, ctx: &AppContext) {
        let mut audio_cache = self.pending_audio_by_correlation_id.lock().await;
        let mut cover_cache = self.pending_cover_by_correlation_id.lock().await;

        // 收集需要处理的项
        let items_to_process: Vec<_> = audio_cache
            .iter()
            .filter_map(|(correlation_id, audio_id)| {
                cover_cache
                    .get(correlation_id)
                    .map(|cover_id| (correlation_id.clone(), audio_id.clone(), cover_id.clone()))
            })
            .collect();

        // 清理已匹配的项
        for (correlation_id, _, _) in &items_to_process {
            audio_cache.remove(correlation_id);
            cover_cache.remove(correlation_id);
        }

        // 释放锁
        drop(audio_cache);
        drop(cover_cache);

        // 执行绑定操作
        for (_correlation_id, audio_id, cover_id) in items_to_process {
            let cmd = BindCmd {
                audio_file_id: audio_id.clone(),
                cover_art_id: cover_id.clone(),
            };
            let ctx = ctx.inherit();
            //info!("Binding audio file to cover art: {}", audio_id);
            if let Err(e) = self.cover_art_service.bind(&ctx, cmd).await {
                error!("Failed to bind audio file to cover art: {}", e);
            } else {
                //info!("Bound audio file to cover art: {}", audio_id);
            }
        }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<AudioFileEvent> for BindToCoverArtCoordinator<B> {
    async fn handle(&self, event: &EventEnvelope<AudioFileEvent>) {
        let ctx = AppContext::from(event);
        match &event.payload.kind {
            AudioFileEventKind::Created(evt) => {
                if !evt.has_cover_art {
                    return;
                }
                // cache audio by correlation id
                {
                    let mut audio_cache = self.pending_audio_by_correlation_id.lock().await;
                    audio_cache.insert(
                        event.correlation_id.clone(),
                        event.payload.audio_file_id.clone(),
                    );
                } // 释放锁
                self.check_and_bind(&ctx).await;
            }
            _ => return,
        }
    }
}

#[async_trait::async_trait]
impl<B: EventBus> Handler<CoverArtEvent> for BindToCoverArtCoordinator<B> {
    async fn handle(&self, event: &EventEnvelope<CoverArtEvent>) {
        let ctx = AppContext::from(event);
        match &event.payload.kind {
            CoverArtEventKind::Created(created) => {
                if created.source != CoverSourceType::Embedded {
                    return;
                }
                // cache cover by correlation id
                {
                    let mut cover_cache = self.pending_cover_by_correlation_id.lock().await;
                    cover_cache.insert(event.correlation_id.clone(), created.cover_art_id.clone());
                } // 释放锁
                self.check_and_bind(&ctx).await;
            }
            _ => {}
        }
    }
}
