/*
pub mod artist_info;
pub mod artwork_id;
pub mod bookmark;
pub mod discs;
pub mod genre;
pub mod kind;
pub mod library;
pub mod lyrics;
pub mod mbz_album;
pub mod player;
pub mod radio;
pub mod scrobble;
pub mod transcoding;
pub mod user;
*/
pub mod album;
pub mod album_location;
pub mod album_stats;
pub mod artist;
pub mod artist_location;
pub mod audio_file;
pub mod genre;
pub mod music_folder;
pub mod participant_stats;
pub mod playback_history;
pub mod play_queue;
pub mod playlist;
pub mod scan_status;
pub mod shared;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("Projection error: {0}")]
    ProjectionError(String),
    #[error("Database error: {0}")]
    DbErr(String),
}
