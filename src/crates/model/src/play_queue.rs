use super::playlist::PlaylistAudioFile;
use serde::{Deserialize, Serialize};

/// Play queue with full audio file details (for getPlayQueue API response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayQueue {
    /// Current playing audio file ID
    pub current_id: Option<i64>,
    /// Position in milliseconds within the currently playing song
    pub position: i64,
    /// Username of the queue owner
    pub username: String,
    /// Client name that last changed this queue
    pub changed_by: String,
    /// Last update timestamp (ISO 8601 format)
    pub changed: String,
    /// Audio files in the queue
    pub entries: Vec<PlaylistAudioFile>,
}
