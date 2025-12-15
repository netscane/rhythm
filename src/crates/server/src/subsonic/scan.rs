use crate::subsonic::response::error::SubsonicError;
use crate::subsonic::response::scan::ScanStatus as ScanStatusResponse;
use crate::subsonic::response::Subsonic;
use crate::AppState;
use actix_web::web;
use application::command::library::{LibraryCommandService, ScanLibraryCmd};
use application::context::AppContext;
use application::query::dao::MusicFolderDao;
use infra::file_type_detector::DefaultFileTypeDetector;
use infra::repository::postgres::command::library::LibraryRepositoryImpl;
use infra::repository::postgres::query::music_folder::MusicFolderDaoImpl;
use infra::storage::factory::StorageClientFactoryImpl;
use std::sync::Arc;

/// OpenSubsonic startScan API - scans all music folders
pub async fn start_library_scan(state: web::Data<AppState>) -> Result<Subsonic, SubsonicError> {
    let state = state.into_inner();

    // Query all music folders
    let music_folder_dao = MusicFolderDaoImpl::new(state.db.clone());
    let folders = music_folder_dao
        .get_all()
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    if folders.is_empty() {
        return Ok(ScanStatusResponse {
            scanning: false,
            count: 0,
            folder_count: 0,
            last_scan: None,
        }
        .into());
    }

    let library_repo = Arc::new(LibraryRepositoryImpl::new(state.db.clone()));
    let scanner_factory = Arc::new(StorageClientFactoryImpl::new());
    let file_type_detector = Arc::new(DefaultFileTypeDetector::new());
    let event_bus = Arc::new(state.event_bus.clone());
    let id_generator = state.id_generator.clone();

    let svc = LibraryCommandService::new(
        library_repo,
        scanner_factory,
        file_type_detector,
        event_bus,
        id_generator,
    );

    let ctx = AppContext::new();

    // Scan all libraries
    for folder in &folders {
        let library_id = folder.id.into();
        let _ = svc
            .scan_library(
                &ctx,
                ScanLibraryCmd {
                    library_id,
                    is_full_scan: true,
                },
            )
            .await;
    }

    Ok(ScanStatusResponse {
        scanning: true,
        count: 0,
        folder_count: folders.len() as i32,
        last_scan: None,
    }
    .into())
}

/// OpenSubsonic getScanStatus API - returns aggregated scan status
pub async fn get_scan_status(state: web::Data<AppState>) -> Result<Subsonic, SubsonicError> {
    let state = state.into_inner();

    // Get all scan statuses
    let all_statuses = state
        .scan_repo
        .get_all_scan_statuses()
        .await
        .unwrap_or_default();

    if all_statuses.is_empty() {
        return Ok(ScanStatusResponse {
            scanning: false,
            count: 0,
            folder_count: 0,
            last_scan: None,
        }
        .into());
    }

    // Aggregate status from all libraries
    let mut scanning = false;
    let mut total_count: i64 = 0;

    for status in all_statuses.values() {
        if status.scanning {
            scanning = true;
        }
        total_count += status.processed_files;
    }

    Ok(ScanStatusResponse {
        scanning,
        count: total_count as i32,
        folder_count: all_statuses.len() as i32,
        last_scan: Some(chrono::Utc::now().naive_utc()),
    }
    .into())
}
