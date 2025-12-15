use crate::error::AppError;
use domain::audio_file::AudioFileEvent;
use domain::audio_file::AudioFileEventKind;
use domain::library::{ScanEnded, ScanStarted};
use model::scan_status::{ScanStatus, ScanStatusRepository};
use std::sync::Arc;

/// ScanStatusProjector trait 用于投影事件到扫描状态
#[async_trait::async_trait]
pub trait ScanStatusProjector: Send + Sync {
    async fn on_audio_file_event(&self, event: &AudioFileEvent) -> Result<(), AppError>;
    async fn on_scan_started(&self, event: &ScanStarted) -> Result<(), AppError>;
    async fn on_scan_ended(&self, event: &ScanEnded) -> Result<(), AppError>;
}

/// ScanStatusProjectorImpl 扫描状态投影器实现
pub struct ScanStatusProjectorImpl {
    repository: Arc<dyn ScanStatusRepository + Send + Sync>,
}

impl ScanStatusProjectorImpl {
    pub fn new(repository: Arc<dyn ScanStatusRepository + Send + Sync>) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl ScanStatusProjector for ScanStatusProjectorImpl {
    async fn on_audio_file_event(&self, event: &AudioFileEvent) -> Result<(), AppError> {
        match &event.kind {
            AudioFileEventKind::Created(created) => {
                // 文件创建时增加处理计数
                if let Ok(Some(mut status)) =
                    self.repository.get_scan_status(&created.library_id).await
                {
                    status.increment_processed();
                    self.repository.save(&status).await?;
                } else {
                    // Initialize if missing
                    let mut status = ScanStatus::new(created.library_id.clone());
                    status.start_scanning(0);
                    status.increment_processed();
                    self.repository.save(&status).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn on_scan_started(&self, event: &ScanStarted) -> Result<(), AppError> {
        let mut status = ScanStatus::new(event.library_id.clone());
        status.start_scanning(0); // 初始化为0，后续会根据实际文件数量更新
        self.repository.save(&status).await?;
        Ok(())
    }

    async fn on_scan_ended(&self, event: &ScanEnded) -> Result<(), AppError> {
        if let Ok(Some(mut status)) = self.repository.get_scan_status(&event.library_id).await {
            status.finish_scanning();
            self.repository.save(&status).await?;
        }
        Ok(())
    }
}
