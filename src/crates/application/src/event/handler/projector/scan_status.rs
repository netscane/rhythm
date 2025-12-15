use crate::event::event_bus::{EventEnvelope, Handler};
use crate::projector::scan_status::ScanStatusProjector;
use domain::audio_file::AudioFileEvent;
use domain::library::LibraryEvent;
use log::error;
use std::sync::Arc;

/// AudioFileEventHandler 音频文件事件处理器
pub struct ScanStatusEventHandler {
    projector: Arc<dyn ScanStatusProjector + Send + Sync>,
}

impl ScanStatusEventHandler {
    pub fn new(projector: Arc<dyn ScanStatusProjector + Send + Sync>) -> Self {
        Self { projector }
    }
}

#[async_trait::async_trait]
impl Handler<AudioFileEvent> for ScanStatusEventHandler {
    async fn handle(&self, envelope: &EventEnvelope<AudioFileEvent>) {
        if let Err(e) = self.projector.on_audio_file_event(&envelope.payload).await {
            error!("Error projecting audio file event: {}", e);
        }
    }
}

/// ScanLifecycleEventHandler 库扫描生命周期事件处理器
pub struct ScanLifecycleEventHandler {
    projector: Arc<dyn ScanStatusProjector + Send + Sync>,
}

impl ScanLifecycleEventHandler {
    pub fn new(projector: Arc<dyn ScanStatusProjector + Send + Sync>) -> Self {
        Self { projector }
    }
}

#[async_trait::async_trait]
impl Handler<LibraryEvent> for ScanLifecycleEventHandler {
    async fn handle(&self, envelope: &EventEnvelope<LibraryEvent>) {
        match &envelope.payload {
            LibraryEvent::ScanStarted(evt) => {
                if let Err(e) = self.projector.on_scan_started(evt).await {
                    error!("Failed to handle scan started event: {}", e);
                }
            }
            LibraryEvent::ScanEnded(evt) => {
                if let Err(e) = self.projector.on_scan_ended(evt).await {
                    error!("Failed to handle scan ended event: {}", e);
                }
            }
            _ => {}
        }
    }
}
