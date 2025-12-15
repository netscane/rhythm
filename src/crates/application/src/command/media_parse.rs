use crate::context::AppContext;
use crate::error::AppError;
use crate::event::event_bus::{CorrelationId, EventBus, EventEnvelope};
use crate::event::events::{AppEvent, AudioFileParsed, ImageFileParsed};
use domain::cover_art::CoverSourceType;
use domain::value::{AudioMetadata, FileMeta, FileType, LibraryId, MediaPath};
use std::path::PathBuf;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait AudioMetadataReader: Send + Sync {
    async fn parse(&self, path: PathBuf) -> Result<AudioMetadata, AppError>;
    async fn get_picture(&self, path: PathBuf) -> Result<Vec<u8>, AppError>;
}

#[async_trait::async_trait]
pub trait StorageClient: Send + Sync {
    async fn get_local_path(&self, path: &MediaPath) -> Result<PathBuf, AppError>;
    async fn list(&self, path: &MediaPath) -> Result<Vec<FileMeta>, AppError>;
    async fn read(&self, path: &MediaPath) -> Result<Vec<u8>, AppError>;
    async fn exists(&self, path: &MediaPath) -> Result<bool, AppError>;
}

#[async_trait::async_trait]
pub trait StorageClientFactory: Send + Sync {
    async fn create(&self, path: &MediaPath) -> Result<Arc<dyn StorageClient>, AppError>;
}

#[derive(Debug)]
pub struct ParseMediaFileCmd {
    pub filemeta: FileMeta,
    pub library_id: LibraryId,
    pub file_type: FileType,
}

#[derive(Clone)]
pub struct MediaFileParseService<B: EventBus> {
    event_bus: Arc<B>,
    storage_client_factory: Arc<dyn StorageClientFactory>,
    audio_metadata_reader: Arc<dyn AudioMetadataReader>,
}

impl<B: EventBus> MediaFileParseService<B> {
    pub fn new(
        event_bus: Arc<B>,
        storage_client_factory: Arc<dyn StorageClientFactory>,
        audio_metadata_reader: Arc<dyn AudioMetadataReader>,
    ) -> Self {
        Self {
            event_bus,
            storage_client_factory,
            audio_metadata_reader,
        }
    }
    async fn parse_audio_file(&self, local_path: &PathBuf) -> Result<AudioMetadata, AppError> {
        let metadata = self.audio_metadata_reader.parse(local_path.clone()).await?;
        Ok(metadata)
    }
    pub async fn parse_media_file(
        &self,
        ctx: &AppContext,
        cmd: ParseMediaFileCmd,
    ) -> Result<(), AppError> {
        let storage_client = self
            .storage_client_factory
            .create(&cmd.filemeta.path)
            .await?;
        let local_path = storage_client.get_local_path(&cmd.filemeta.path).await?;
        let mut app_events = Vec::new();
        match cmd.file_type {
            FileType::Audio => {
                let metadata = self.parse_audio_file(&local_path).await?;
                app_events.push(AppEvent::AudioFileParsed(AudioFileParsed {
                    library_id: cmd.library_id.clone(),
                    metadata: metadata.clone(),
                    file_info: cmd.filemeta.clone(),
                }));
                if metadata.picture.is_some() {
                    app_events.push(AppEvent::ImageFileParsed(ImageFileParsed {
                        library_id: cmd.library_id.clone(),
                        file_info: cmd.filemeta.clone(),
                        source: CoverSourceType::Embedded,
                    }));
                }
            }
            FileType::Image => {
                app_events.push(AppEvent::ImageFileParsed(ImageFileParsed {
                    library_id: cmd.library_id.clone(),
                    file_info: cmd.filemeta.clone(),
                    source: CoverSourceType::External,
                }));
            }
            _ => {
                //log::info!("Unsupported file type: {:?}", cmd.file_type);
            }
        }
        // new a correlation id
        let correlation_id = CorrelationId::new();
        for event in app_events {
            let envelope =
                EventEnvelope::new(0, 0, event, correlation_id.clone(), ctx.event_id.clone());
            self.event_bus.publish(envelope).await?;
        }
        Ok(())
    }
}
