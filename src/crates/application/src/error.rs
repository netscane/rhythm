use domain::album::AlbumError;
use domain::annotation::AnnotationError;
use domain::artist::ArtistError;
use domain::audio_file::AudioFileError;
use domain::cover_art::CoverArtError;
use domain::genre::GenreError;
use domain::library::LibraryError;
use domain::player::PlayerError;
use domain::user::UserError;
use model::ModelError;

//pub mod song;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Repository error: {0}: {1}")]
    RepositoryError(String, String),
    // from AudioFileError
    #[error("Audio file error: {0}")]
    AudioFileError(#[from] AudioFileError),
    #[error("Album error: {0}")]
    AlbumError(#[from] AlbumError),
    #[error("Artist error: {0}")]
    ArtistError(#[from] ArtistError),
    #[error("Genre error: {0}")]
    GenreError(#[from] GenreError),
    #[error("Library error: {0}")]
    LibraryError(#[from] LibraryError),
    #[error("User error: {0}")]
    UserError(#[from] UserError),
    #[error("Annotation error: {0}")]
    AnnotationError(#[from] AnnotationError),
    #[error("Cover art error: {0}")]
    CoverArtError(#[from] CoverArtError),
    #[error("Player error: {0}")]
    PlayerError(#[from] PlayerError),
    #[error("Aggregate not found: {0}: {1}")]
    AggregateNotFound(String, String),

    #[error("Parse audio metadata error: {0}")]
    ParseAudioMetadataError(String),

    #[error("Auth error: {0}")]
    AuthError(String),

    #[error("Invalid cover art format: {0}")]
    InvalidCoverArtFormat(String),

    #[error("Projection error: {0}")]
    ProjectionError(String),

    #[error("Model error: {0}")]
    ModelError(#[from] ModelError),

    #[error("Unknown error: {0}")]
    UnknownError(String),
}
