use application::command::library::{ScanError, Scanner, ScannerFactory};
use application::command::media_parse::{StorageClient, StorageClientFactory};
use async_trait::async_trait;
use domain::value::MediaPath;
use std::sync::Arc;

use super::local::LocalStorageClient;
use super::smb::SmbStorageClient;

#[derive(Debug, Clone)]
pub struct StorageClientFactoryImpl;

impl StorageClientFactoryImpl {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StorageClientFactory for StorageClientFactoryImpl {
    async fn create(
        &self,
        path: &MediaPath,
    ) -> Result<Arc<dyn StorageClient>, application::error::AppError> {
        match path.protocol.as_str() {
            "local" | "" => Ok(Arc::new(LocalStorageClient::new())),
            "smb" => Ok(Arc::new(SmbStorageClient::new())),
            p => Err(application::error::AppError::UnknownError(format!(
                "Unsupported storage protocol: {}",
                p
            ))),
        }
    }
}

#[async_trait]
impl ScannerFactory for StorageClientFactoryImpl {
    async fn create(&self, protocol: &str) -> Result<Arc<dyn Scanner>, ScanError> {
        match protocol {
            "local" | "" => Ok(Arc::new(LocalStorageClient::new())),
            "smb" => Ok(Arc::new(SmbStorageClient::new())),
            _ => Err(ScanError::OtherError(format!(
                "Unsupported scanner protocol: {}",
                protocol
            ))),
        }
    }
}
