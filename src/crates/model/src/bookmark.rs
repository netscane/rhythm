use super::media_file::MediaFile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Bookmarkable {
    pub bookmark_position: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bookmark {
    pub item: MediaFile,
    pub comment: String,
    pub position: i64,
    pub changed_by: String,
    pub created_at: i64,
    pub updated_at: i64,
}
