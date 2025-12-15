use async_trait::async_trait;
use dashmap::DashMap;
use domain::value::LibraryId;
use model::scan_status::{ScanStatus, ScanStatusRepository};
use model::ModelError;
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct InMemoryScanStatusRepository {
    store: std::sync::Arc<DashMap<LibraryId, ScanStatus>>,
}

impl InMemoryScanStatusRepository {
    pub fn new() -> Self {
        Self {
            store: std::sync::Arc::new(DashMap::new()),
        }
    }
}

#[async_trait]
impl ScanStatusRepository for InMemoryScanStatusRepository {
    async fn get_scan_status(
        &self,
        library_id: &LibraryId,
    ) -> Result<Option<ScanStatus>, ModelError> {
        Ok(self.store.get(library_id).map(|v| v.clone()))
    }

    async fn get_all_scan_statuses(&self) -> Result<HashMap<LibraryId, ScanStatus>, ModelError> {
        Ok(self
            .store
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect())
    }

    async fn save(&self, status: &ScanStatus) -> Result<(), ModelError> {
        self.store.insert(status.library_id.clone(), status.clone());
        Ok(())
    }
}
