use crate::event::DomainEvent;
use crate::value::{AnnotationId, UserId};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnnotationError {
    #[error("Repository error: {0}")]
    RepositoryError(String),
    #[error("Unknown kind: {0}")]
    UnknownKind(String),
    #[error("Entity not found: {0}")]
    NotFound(String),
    #[error("Version conflict: expected {expected}, found {actual}")]
    VersionConflict { expected: i64, actual: i64 },
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Domain error: {0}")]
    DomainError(String),
    #[error("Database error: {0}")]
    DbErr(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Default)]
pub enum Kind {
    #[default]
    AudioFile,
    Artist,
    Album,
    Playlist,
}

impl Kind {
    pub fn prefix(&self) -> &'static str {
        match self {
            Kind::AudioFile => "af",
            Kind::Artist => "ar",
            Kind::Album => "al",
            Kind::Playlist => "pl",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Kind::AudioFile => "audio_file",
            Kind::Artist => "artist",
            Kind::Album => "album",
            Kind::Playlist => "playlist",
        }
    }
}

impl FromStr for Kind {
    type Err = AnnotationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "audio_file" | "af" => Ok(Kind::AudioFile),
            "artist" | "ar" => Ok(Kind::Artist),
            "album" | "al" => Ok(Kind::Album),
            "playlist" | "pl" => Ok(Kind::Playlist),
            _ => Err(AnnotationError::UnknownKind(s.to_string())),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// 注解事件枚举
#[derive(Debug, Clone)]
pub enum AnnotationEvent {
    ItemStarred {
        annotation_id: AnnotationId,
        version: i64,
        user_id: UserId,
        item_id: i64,
        item_type: String,
    },
    ItemUnstarred {
        annotation_id: AnnotationId,
        version: i64,
        user_id: UserId,
        item_id: i64,
        item_type: String,
    },
    ItemRated {
        annotation_id: AnnotationId,
        version: i64,
        user_id: UserId,
        item_id: i64,
        item_type: String,
        rating: i32,
    },
    ItemScrobbled {
        annotation_id: AnnotationId,
        version: i64,
        user_id: UserId,
        item_id: i64,
        item_type: String,
    },
}

impl DomainEvent for AnnotationEvent {
    fn aggregate_id(&self) -> i64 {
        match self {
            AnnotationEvent::ItemStarred { annotation_id, .. } => annotation_id.as_i64(),
            AnnotationEvent::ItemUnstarred { annotation_id, .. } => annotation_id.as_i64(),
            AnnotationEvent::ItemRated { annotation_id, .. } => annotation_id.as_i64(),
            AnnotationEvent::ItemScrobbled { annotation_id, .. } => annotation_id.as_i64(),
        }
    }

    fn version(&self) -> i64 {
        match self {
            AnnotationEvent::ItemStarred { version, .. } => *version,
            AnnotationEvent::ItemUnstarred { version, .. } => *version,
            AnnotationEvent::ItemRated { version, .. } => *version,
            AnnotationEvent::ItemScrobbled { version, .. } => *version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub id: AnnotationId,
    pub user_id: UserId,
    pub item_kind: Kind, // AudioFile, Artist, Album, Playlist
    pub item_id: i64,
    pub rating: i32,
    pub starred: bool,
    pub starred_at: NaiveDateTime,
    pub played_count: i32,
    pub played_at: NaiveDateTime,
    pub version: i64,
    pub pending_events: Vec<AnnotationEvent>,
}

impl Annotation {
    pub fn new(id: AnnotationId, user_id: UserId, item_kind: Kind, item_id: i64) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            id,
            user_id,
            item_kind,
            item_id,
            rating: 0,
            starred: false,
            starred_at: now,
            played_count: 0,
            played_at: now,
            version: 0,
            pending_events: Vec::new(),
        }
    }

    pub fn set_star(&mut self) -> Result<(), AnnotationError> {
        if !self.starred {
            self.starred = true;
            self.starred_at = Utc::now().naive_utc();
            self.version += 1;
            self.pending_events.push(AnnotationEvent::ItemStarred {
                annotation_id: self.id.clone(),
                version: self.version,
                user_id: self.user_id.clone(),
                item_id: self.item_id,
                item_type: self.item_kind.name().to_string(),
            });
        }
        Ok(())
    }

    pub fn unset_star(&mut self) -> Result<(), AnnotationError> {
        if self.starred {
            self.starred = false;
            self.starred_at = Utc::now().naive_utc();
            self.version += 1;
            self.pending_events.push(AnnotationEvent::ItemUnstarred {
                annotation_id: self.id.clone(),
                version: self.version,
                user_id: self.user_id.clone(),
                item_id: self.item_id,
                item_type: self.item_kind.name().to_string(),
            });
        }
        Ok(())
    }

    pub fn set_rating(&mut self, rating: i32) -> Result<(), AnnotationError> {
        if rating > 5 {
            return Err(AnnotationError::ValidationError(
                "Rating must be between 0 and 5".to_string(),
            ));
        }

        if self.rating != rating {
            self.rating = rating;
            self.version += 1;
            self.pending_events.push(AnnotationEvent::ItemRated {
                annotation_id: self.id.clone(),
                version: self.version,
                user_id: self.user_id.clone(),
                item_id: self.item_id,
                item_type: self.item_kind.name().to_string(),
                rating,
            });
        }
        Ok(())
    }

    pub fn scrobble(&mut self) -> Result<(), AnnotationError> {
        self.played_count += 1;
        self.played_at = Utc::now().naive_utc();
        self.version += 1;
        self.pending_events.push(AnnotationEvent::ItemScrobbled {
            annotation_id: self.id.clone(),
            version: self.version,
            user_id: self.user_id.clone(),
            item_id: self.item_id,
            item_type: self.item_kind.name().to_string(),
        });
        Ok(())
    }

    // 从事件队列中拉取所有事件
    pub fn pop_events(&mut self) -> Vec<AnnotationEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

// 仓储接口
#[async_trait]
pub trait AnnotationRepository: Send + Sync {
    async fn find_by_item(
        &self,
        item_kind: Kind,
        item_id: i64,
    ) -> Result<Option<Annotation>, AnnotationError>;
    async fn find_by_item_id(&self, item_id: i64) -> Result<Option<Annotation>, AnnotationError>;
    async fn save(&self, annotation: Annotation) -> Result<(), AnnotationError>;
    async fn find_by_user_id(&self, user_id: UserId) -> Result<Vec<Annotation>, AnnotationError>;
    async fn delete_all(&self) -> Result<(), AnnotationError>;
    async fn delete(&self, annotation: Annotation) -> Result<(), AnnotationError>;
}
