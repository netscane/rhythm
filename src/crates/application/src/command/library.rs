use super::shared::IdGenerator;
use crate::context::AppContext;
use crate::error::AppError;
use crate::event::event_bus::{CorrelationId, EventBus, EventEnvelope};
use async_trait::async_trait;
use domain::library::{LibraryEvent, LibraryItem, LibraryRepository};
use domain::value::LibraryId;
use domain::value::{FileMeta, FileType};
use log::{error, info};
use std::sync::Arc;
use thiserror::Error;
use tokio;
use tokio::sync::mpsc;
use tokio::time::Instant;
// 存储后端错误
#[derive(Error, Debug)]
pub enum ScanError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("No more items to scan")]
    NoMoreItems,
    #[error("Other error: {0}")]
    OtherError(String),
}

#[async_trait::async_trait]
pub trait FileTypeDetector: Send + Sync {
    fn detect(&self, suffix: &str) -> FileType;
}

#[async_trait]
pub trait ScannerFactory: Send + Sync {
    async fn create(&self, protocol: &str) -> Result<Arc<dyn Scanner>, ScanError>;
}

type FileMetaResult = Result<FileMeta, ScanError>;

// 存储后端基础设施trait
#[async_trait]
pub trait Scanner: Send + Sync {
    async fn scan(&self, root: &str) -> Result<mpsc::Receiver<FileMetaResult>, ScanError>;
}

pub trait ScanFactory: Send + Sync {
    fn create(&self, uri: &str) -> Result<Arc<dyn Scanner>, ScanError>;
}

pub struct ScanLibraryCmd {
    pub library_id: LibraryId,
    pub is_full_scan: bool,
}

pub struct LibraryCommandService<T, B> {
    library_repo: Arc<T>,
    scanner_factory: Arc<dyn ScannerFactory>,
    file_type_detector: Arc<dyn FileTypeDetector>,
    event_bus: Arc<B>,
    id_generator: Arc<dyn IdGenerator>,
}

impl<T, B> LibraryCommandService<T, B>
where
    T: LibraryRepository + Clone + Send + Sync + 'static,
    B: EventBus + Clone + Send + Sync + 'static,
{
    pub fn new(
        library_repository: Arc<T>,
        scanner_factory: Arc<dyn ScannerFactory>,
        file_type_detector: Arc<dyn FileTypeDetector>,
        event_bus: Arc<B>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            library_repo: library_repository,
            scanner_factory: scanner_factory.clone(),
            file_type_detector: file_type_detector.clone(),
            event_bus: event_bus.clone(),
            id_generator: id_generator.clone(),
        }
    }
    pub async fn scan_library(
        &self,
        context: &AppContext,
        cmd: ScanLibraryCmd,
    ) -> Result<(), AppError> {
        info!("Scan library: {}", cmd.library_id);
        let library_id = cmd.library_id;
        let mut library =
            self.library_repo
                .find_by_id(&library_id)
                .await?
                .ok_or(AppError::AggregateNotFound(
                    "Library".to_string(),
                    library_id.to_string(),
                ))?;
        library.start_scan(cmd.is_full_scan)?;
        self.library_repo.save(&library).await?;
        for event in library.take_events() {
            let envelope = EventEnvelope::<LibraryEvent>::new_with_domain_event(
                event,
                CorrelationId::new(),
                context.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }

        let scanner_factory = Arc::clone(&self.scanner_factory);
        let event_bus = Arc::clone(&self.event_bus);
        let library_repo = Arc::clone(&self.library_repo);
        let id_generator = Arc::clone(&self.id_generator);
        let file_type_detector = Arc::clone(&self.file_type_detector);
        let context = context.clone();
        tokio::spawn(async move {
            let scanner = scanner_factory
                .create(&library.path.protocol)
                .await
                .or_else(|e| {
                    error!("Failed to create scanner: {}", e);
                    Err(e)
                });
            if let Ok(scanner) = scanner {
                let start_time = Instant::now();
                info!("Scanner created: {}", library.path.path);
                if let Ok(mut receiver) = scanner.scan(&library.path.path).await {
                    let mut scan_err = None;
                    let mut scanned_count = 0u64;
                    let mut last_log_time = Instant::now();
                    let mut last_log_count = 0u64;

                    while let Some(result) = receiver.recv().await {
                        match result {
                            Ok(file) => {
                                let item_id = id_generator.next_id().await.unwrap();
                                let file_type = file_type_detector.detect(&file.suffix);
                                let library_item = LibraryItem::new(
                                    item_id.into(),
                                    library_id.clone(),
                                    file,
                                    file_type,
                                );
                                library.add_item(library_item);
                                let events = library.take_events();
                                for event in events {
                                    //info!("Published event: {:?}", event);
                                    let envelope =
                                        EventEnvelope::<LibraryEvent>::new_with_domain_event(
                                            event,
                                            CorrelationId::new(),
                                            context.event_id.clone(),
                                        );
                                    if let Err(e) = event_bus.publish(envelope).await {
                                        error!("Failed to publish event: {}", e);
                                    }
                                }

                                scanned_count += 1;

                                // 每1000个文件打印一次进度日志
                                if scanned_count % 1000 == 0 {
                                    let elapsed = last_log_time.elapsed();
                                    let batch_count = scanned_count - last_log_count;
                                    let speed = batch_count as f64 / elapsed.as_secs_f64();
                                    let total_elapsed = start_time.elapsed();
                                    let avg_speed =
                                        scanned_count as f64 / total_elapsed.as_secs_f64();

                                    info!(
                                        "Scan progress: {} files scanned, batch speed: {:.2} files/s, avg speed: {:.2} files/s, total time: {:.2}s",
                                        scanned_count,
                                        speed,
                                        avg_speed,
                                        total_elapsed.as_secs_f64()
                                    );

                                    last_log_time = Instant::now();
                                    last_log_count = scanned_count;
                                }
                            }
                            Err(e) => {
                                error!("Failed to scan library: {}", e);
                                scan_err = Some(e);
                                break;
                            }
                        }
                    }
                    if let Some(_e) = scan_err {
                        library.abort_scan();
                    } else {
                        library.finish_scan();
                    }

                    if let Err(e) = library_repo.save(&library).await {
                        error!("Failed to save library: {}", e);
                    }
                    for event in library.take_events() {
                        let envelope = EventEnvelope::<LibraryEvent>::new_with_domain_event(
                            event,
                            CorrelationId::new(),
                            context.event_id.clone(),
                        );
                        if let Err(e) = event_bus.publish(envelope).await {
                            error!("Failed to publish event: {}", e);
                        }
                    }

                    let total_elapsed = start_time.elapsed();
                    let avg_speed = scanned_count as f64 / total_elapsed.as_secs_f64();
                    info!(
                        "Scan finished: {}, total files: {}, total time: {:.2}s, avg speed: {:.2} files/s",
                        library.path.path,
                        scanned_count,
                        total_elapsed.as_secs_f64(),
                        avg_speed
                    );
                } else {
                    error!(
                        "Failed to create storage backend for library {}",
                        library_id
                    );
                }
            } else {
                error!("Failed to create scanner for library {}", library_id);
            }
        });

        Ok(())
    }
}
