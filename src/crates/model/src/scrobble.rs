use super::media_file::MediaFile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrobbleEntry {
    pub id: String,
    pub service: String,
    pub user_id: String,
    pub play_time: i64,
    pub enqueue_time: i64,
    pub media_file: MediaFile,
}
