use thiserror::Error;

pub mod artist;
pub mod config;
pub mod dao;
pub mod dto;
pub mod get_album;
pub mod get_album_info;
pub mod get_album_list;
pub mod get_artist;
pub mod get_artist_info;
pub mod get_artist_list;
pub mod get_cover_art;
pub mod get_genres;
pub mod get_music_folders;
pub mod get_play_queue;
pub mod get_playlist;
pub mod get_random_songs;
pub mod get_songs_list;
pub mod get_similar_songs;
pub mod get_song;
pub mod get_songs_by_genre;
pub mod get_starred;
pub mod get_top_songs;
pub mod shared;
pub mod stream_cache;
pub mod stream_media;

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("Database error: {0}")]
    DbError(String),
}
