use crate::ModelError;
use domain::value::LibraryId;
use std::collections::HashMap;

/// ScanStatus 扫描状态，支持多库扫描
#[derive(Debug, Clone)]
pub struct ScanStatus {
    pub library_id: LibraryId,
    pub scanning: bool,
    pub count: i64,
    pub total_files: i64,
    pub processed_files: i64,
    pub error_count: i64,
}

impl ScanStatus {
    pub fn new(library_id: LibraryId) -> Self {
        Self {
            library_id,
            scanning: false,
            count: 0,
            total_files: 0,
            processed_files: 0,
            error_count: 0,
        }
    }

    /// 开始扫描
    pub fn start_scanning(&mut self, total_files: i64) {
        self.scanning = true;
        self.total_files = total_files;
        self.processed_files = 0;
        self.error_count = 0;
    }

    /// 完成扫描
    pub fn finish_scanning(&mut self) {
        self.scanning = false;
    }

    /// 增加处理文件计数
    pub fn increment_processed(&mut self) {
        self.processed_files += 1;
    }

    /// 增加错误计数
    pub fn increment_error(&mut self) {
        self.error_count += 1;
    }

    /// 获取扫描进度百分比
    pub fn get_progress_percentage(&self) -> f64 {
        if self.total_files == 0 {
            return 0.0;
        }
        (self.processed_files as f64 / self.total_files as f64) * 100.0
    }
}

#[async_trait::async_trait]
pub trait ScanStatusRepository {
    async fn get_scan_status(
        &self,
        library_id: &LibraryId,
    ) -> Result<Option<ScanStatus>, ModelError>;
    async fn get_all_scan_statuses(&self) -> Result<HashMap<LibraryId, ScanStatus>, ModelError>;
    async fn save(&self, status: &ScanStatus) -> Result<(), ModelError>;
}
