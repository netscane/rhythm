use crate::event::DomainEvent;
use crate::value::{AlbumId, ArtistId, GenreId};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Clone)]
pub enum ArtistEvent {
    Created(ArtistCreated),
    Found(ArtistFound),
    GenreUpdated(ArtistGenreUpdated),
    Removed(ArtistRemoved),
}

#[derive(Clone)]
pub struct ArtistCreated {
    pub artist_id: ArtistId,
    pub version: i64,
    pub name: String,
    pub sort_name: String,
}

#[derive(Clone)]
pub struct ArtistFound {
    pub artist_id: ArtistId,
    pub version: i64,
    pub name: String,
    pub sort_name: String,
}

#[derive(Clone)]
pub struct ArtistGenreUpdated {
    pub artist_id: ArtistId,
    pub version: i64,
    pub name: String,
    pub sort_name: String,
    pub genres: Vec<GenreId>,
}

#[derive(Clone)]
pub struct ArtistRemoved {
    pub artist_id: ArtistId,
    pub version: i64,
    pub name: String,
    pub sort_name: String,
}

impl DomainEvent for ArtistEvent {
    fn aggregate_id(&self) -> i64 {
        match self {
            ArtistEvent::Created(event) => event.artist_id.as_i64(),
            ArtistEvent::Found(event) => event.artist_id.as_i64(),
            ArtistEvent::GenreUpdated(event) => event.artist_id.as_i64(),
            ArtistEvent::Removed(event) => event.artist_id.as_i64(),
        }
    }
    fn version(&self) -> i64 {
        match self {
            ArtistEvent::Created(event) => event.version,
            ArtistEvent::Found(event) => event.version,
            ArtistEvent::GenreUpdated(event) => event.version,
            ArtistEvent::Removed(event) => event.version,
        }
    }
}

#[derive(Error, Debug)]
pub enum ArtistError {
    #[error("{0}")]
    DbErr(String),
    #[error("{0}")]
    OtherErr(String),
    #[error("Version conflict: {0}")]
    VersionConflict(i64),
}

/// Artist status enum for state management
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    New,
    Active,
    Deleted,
}

impl Default for Status {
    fn default() -> Self {
        Status::New
    }
}

#[derive(Clone)]
pub struct Artist {
    pub id: ArtistId,
    pub name: String,
    pub sort_name: String,
    pub mbz_artist_id: Option<String>,
    pub biography: Option<String>,
    pub version: i64,
    pub genre: Option<GenreId>,
    pub genres: Vec<GenreId>,
    pub pending_events: Vec<ArtistEvent>,
}

impl Artist {
    pub fn new(id: ArtistId, name: String, sort_name: String) -> Self {
        let mut artist = Self {
            id,
            name,
            sort_name,
            mbz_artist_id: None,
            biography: None,
            version: 0,
            genre: None,
            genres: Vec::new(),
            pending_events: Vec::new(),
        };
        artist
            .pending_events
            .push(ArtistEvent::Created(ArtistCreated {
                artist_id: artist.id.clone(),
                version: artist.version,
                name: artist.name.clone(),
                sort_name: artist.sort_name.clone(),
            }));
        artist
    }

    pub fn bind_to_genre(&mut self, genre_id: GenreId) -> Result<(), ArtistError> {
        if self.genre.is_none() {
            self.genre = Some(genre_id.clone());
        }
        if !self.genres.contains(&genre_id) {
            self.genres.push(genre_id);
            self.version += 1;
            self.pending_events
                .push(ArtistEvent::GenreUpdated(ArtistGenreUpdated {
                    artist_id: self.id.clone(),
                    version: self.version,
                    name: self.name.clone(),
                    sort_name: self.sort_name.clone(),
                    genres: self.genres.clone(),
                }));
        }
        Ok(())
    }

    pub fn take_events(&mut self) -> Vec<ArtistEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

#[async_trait]
pub trait ArtistRepository: Send + Sync {
    async fn find_by_sort_name(&self, sort_name: &String) -> Result<Option<Artist>, ArtistError>;
    async fn save(&self, mut artist: Artist) -> Result<Artist, ArtistError>;
    async fn delete(&self, artist_id: ArtistId) -> Result<(), ArtistError>;
    async fn by_id(&self, id: ArtistId) -> Result<Option<Artist>, ArtistError>;
}
