use crate::event::DomainEvent;
use crate::value::{FileType, LibraryId, LibraryItemId, MediaPath};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::HashMap;
use thiserror::Error;

use super::value::FileMeta;

#[async_trait]
pub trait LibraryRepository: Send + Sync {
    async fn save(&self, library: &Library) -> Result<(), LibraryError>;
    async fn find_by_id(&self, id: &LibraryId) -> Result<Option<Library>, LibraryError>;
}

#[derive(Error, Debug)]
pub enum LibraryError {
    #[error("Library is currently being scanned")]
    ScanningInProgress,
    #[error("Library not found")]
    NotFound,
    #[error("Database error: {0}")]
    DbError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

#[derive(Debug, Clone, PartialEq)]
#[repr(i32)]
pub enum ScanStatus {
    Idle = 1,
    Scanning = 2,
}

impl From<ScanStatus> for i32 {
    fn from(value: ScanStatus) -> Self {
        value as i32
    }
}

impl TryFrom<i32> for ScanStatus {
    type Error = String;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ScanStatus::Idle),
            2 => Ok(ScanStatus::Scanning),
            _ => Err(format!("invalid value:{}", value)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LibraryEvent {
    FileAdded(FileAdded),
    FileUpdated(FileUpdated),
    FileRemoved(FileRemoved),
    ScanStarted(ScanStarted),
    ScanEnded(ScanEnded),
}

impl DomainEvent for LibraryEvent {
    fn aggregate_id(&self) -> i64 {
        match self {
            LibraryEvent::FileAdded(event) => event.library_id.as_i64(),
            LibraryEvent::FileUpdated(event) => event.library_id.as_i64(),
            LibraryEvent::FileRemoved(event) => event.library_id.as_i64(),
            LibraryEvent::ScanStarted(event) => event.library_id.as_i64(),
            LibraryEvent::ScanEnded(event) => event.library_id.as_i64(),
        }
    }

    fn version(&self) -> i64 {
        match self {
            LibraryEvent::FileAdded(event) => event.version,
            LibraryEvent::FileUpdated(event) => event.version,
            LibraryEvent::FileRemoved(event) => event.version,
            LibraryEvent::ScanStarted(event) => event.version,
            LibraryEvent::ScanEnded(event) => event.version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileAdded {
    pub library_id: LibraryId,
    pub version: i64,
    pub item: LibraryItem,
}

#[derive(Debug, Clone)]
pub struct FileUpdated {
    pub library_id: LibraryId,
    pub version: i64,
    pub item: LibraryItem,
}
#[derive(Debug, Clone)]
pub struct FileRemoved {
    pub library_id: LibraryId,
    pub version: i64,
    pub path: MediaPath,
}

#[derive(Debug, Clone)]
pub struct ScanStarted {
    pub library_id: LibraryId,
    pub version: i64,
}

#[derive(Debug, Clone)]
pub struct ScanEnded {
    pub library_id: LibraryId,
    pub version: i64,
}

#[derive(Debug, Clone)]
pub struct LibraryItem {
    pub id: LibraryItemId,
    pub library_id: LibraryId,
    pub path: MediaPath,
    pub size: i64,
    pub suffix: String,
    pub mtime: NaiveDateTime,
    pub atime: NaiveDateTime,
    pub state: LibraryItemState,
    pub file_type: FileType,
}

impl LibraryItem {
    pub fn new(
        id: LibraryItemId,
        library_id: LibraryId,
        file: FileMeta,
        file_type: FileType,
    ) -> Self {
        Self {
            id,
            library_id,
            path: file.path,
            size: file.size,
            suffix: file.suffix.clone(),
            mtime: file.mtime,
            atime: file.atime,
            state: LibraryItemState::New,
            file_type,
        }
    }
}

impl From<LibraryItem> for FileMeta {
    fn from(item: LibraryItem) -> Self {
        Self {
            path: item.path.clone(),
            dir_path: item.path.clone(),
            size: item.size,
            suffix: item.suffix.clone(),
            mtime: item.mtime,
            atime: item.atime,
            ctime: item.mtime,
            hash: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LibraryItemState {
    New,
    Deleted,
    Updated,
    Origin,
}

impl From<LibraryItemState> for i32 {
    fn from(value: LibraryItemState) -> Self {
        match value {
            LibraryItemState::New => 0,
            LibraryItemState::Deleted => 1,
            LibraryItemState::Updated => 2,
            LibraryItemState::Origin => 3,
        }
    }
}

impl TryFrom<i32> for LibraryItemState {
    type Error = String;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(LibraryItemState::New),
            1 => Ok(LibraryItemState::Deleted),
            2 => Ok(LibraryItemState::Updated),
            3 => Ok(LibraryItemState::Origin),
            _ => Err(format!("invalid value:{}", value)),
        }
    }
}

impl PartialEq for LibraryItem {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

#[derive(Clone, Debug)]
pub struct Library {
    pub id: LibraryId,
    pub name: String,
    pub path: MediaPath,
    pub items: HashMap<String, LibraryItem>,
    pub scan_status: ScanStatus,
    pub version: i64,
    pub last_scan_at: NaiveDateTime,
    pub pending_events: Vec<LibraryEvent>,
}

impl Library {
    pub fn new(id: LibraryId, name: String, path: MediaPath) -> Self {
        Library {
            id,
            name,
            path,
            items: HashMap::new(),
            scan_status: ScanStatus::Idle,
            version: 0,
            last_scan_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            pending_events: Vec::new(),
        }
    }

    pub fn start_scan(&mut self, full_scan: bool) -> Result<(), LibraryError> {
        if self.scan_status == ScanStatus::Scanning {
            return Err(LibraryError::ScanningInProgress);
        }
        self.scan_status = ScanStatus::Scanning;
        if full_scan {
            self.last_scan_at = DateTime::from_timestamp(0, 0).unwrap().naive_utc();
        }
        self.items.iter_mut().for_each(|(_, item)| {
            item.state = LibraryItemState::Deleted;
            self.version += 1;
        });
        self.pending_events
            .push(LibraryEvent::ScanStarted(ScanStarted {
                library_id: self.id.clone(),
                version: self.version,
            }));
        Ok(())
    }

    pub fn finish_scan(&mut self) {
        if self.scan_status != ScanStatus::Idle {
            self.scan_status = ScanStatus::Idle;
            self.last_scan_at = Utc::now().naive_utc();
        }
        let mut items_to_remove = Vec::new();
        self.items
            .iter()
            .filter(|(_, item)| item.state == LibraryItemState::Deleted)
            .for_each(|(path, _)| {
                items_to_remove.push(path.clone());
                self.pending_events
                    .push(LibraryEvent::FileRemoved(FileRemoved {
                        library_id: self.id.clone(),
                        version: self.version,
                        path: MediaPath {
                            protocol: self.path.protocol.clone(),
                            path: path.clone(),
                        },
                    }));
            });
        for path in items_to_remove {
            self.items.remove(&path);
        }
        self.pending_events.push(LibraryEvent::ScanEnded(ScanEnded {
            library_id: self.id.clone(),
            version: self.version,
        }));
    }
    pub fn abort_scan(&mut self) {
        if self.scan_status == ScanStatus::Scanning {
            self.scan_status = ScanStatus::Idle;
            self.pending_events.push(LibraryEvent::ScanEnded(ScanEnded {
                library_id: self.id.clone(),
                version: self.version,
            }));
            self.items.iter_mut().for_each(|(_, item)| {
                item.state = LibraryItemState::Origin;
                self.version += 1;
            });
        }
    }
    pub fn add_item(&mut self, item: LibraryItem) {
        if let Some(item) = self.items.get_mut(&item.path.path) {
            if item.mtime != item.mtime || item.atime != item.atime {
                item.state = LibraryItemState::Updated;
                item.mtime = item.mtime;
                item.atime = item.atime;
                self.pending_events
                    .push(LibraryEvent::FileUpdated(FileUpdated {
                        library_id: self.id.clone(),
                        version: self.version,
                        item: item.clone(),
                    }));
            } else {
                item.state = LibraryItemState::Origin;
            }
        } else {
            self.items.insert(item.path.path.clone(), item.clone());
            self.pending_events.push(LibraryEvent::FileAdded(FileAdded {
                library_id: self.id.clone(),
                version: self.version,
                item: item.clone(),
            }));
        }
        self.version += 1;
    }

    pub fn take_events(&mut self) -> Vec<LibraryEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
