use crate::event::DomainEvent;
use crate::value::AudioFileId;
use crate::value::{PlayQueueId, UserId};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlayQueueError {
    #[error("Version conflict: expected {expected}, got {actual}")]
    VersionConflict { expected: i32, actual: i32 },
    #[error("Database error: {0}")]
    DbErr(String),
    #[error("{0}")]
    OtherErr(String),
}

#[derive(Debug, Clone)]
pub struct PlayQueue {
    pub id: PlayQueueId,
    pub name: String,
    pub user_id: UserId,
    pub items: Vec<AudioFileId>,
    pub current_index: Option<usize>,
    /// Position in milliseconds within the currently playing song
    pub position: i64,
    /// Client name that last changed this queue
    pub changed_by: String,
    pub updated_at: i64,
    pub pending_events: Vec<PlayQueueDomainEvent>,
}

impl PlayQueue {
    pub fn new(id: PlayQueueId, user_id: UserId, changed_by: String) -> Self {
        Self {
            id,
            name: String::new(),
            user_id,
            items: vec![],
            current_index: None,
            position: 0,
            changed_by,
            updated_at: chrono::Utc::now().timestamp(),
            pending_events: vec![],
        }
    }

    /// Create a play queue from saved state (for savePlayQueue API)
    pub fn from_saved_state(
        id: PlayQueueId,
        user_id: UserId,
        items: Vec<AudioFileId>,
        current: Option<AudioFileId>,
        position: i64,
        changed_by: String,
    ) -> Self {
        let current_index = current.as_ref().and_then(|c| items.iter().position(|i| i == c));
        Self {
            id,
            name: String::new(),
            user_id,
            items,
            current_index,
            position,
            changed_by,
            updated_at: chrono::Utc::now().timestamp(),
            pending_events: vec![],
        }
    }

    fn record(&mut self, kind: PlayQueueEventKind) {
        let event = PlayQueueDomainEvent {
            queue_id: self.id.clone(),
            kind,
        };
        self.pending_events.push(event);
    }

    pub fn add_item(&mut self, audio_file_id: AudioFileId) {
        self.items.push(audio_file_id.clone());
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
        self.record(PlayQueueEventKind::AudioFileAddedToQueue {
            queue_id: self.id.clone(),
            audio_file_id,
        });
    }

    pub fn remove_item(&mut self, audio_file_id: &AudioFileId) {
        if let Some(pos) = self.items.iter().position(|x| x == audio_file_id) {
            self.items.remove(pos);
            // Adjust current_index if needed
            if let Some(ci) = self.current_index {
                if self.items.is_empty() {
                    self.current_index = None;
                } else if pos < ci {
                    self.current_index = Some(ci.saturating_sub(1));
                } else if pos == ci {
                    // Keep at same index if possible
                    let new_ci = if ci >= self.items.len() {
                        self.items.len() - 1
                    } else {
                        ci
                    };
                    self.current_index = Some(new_ci);
                }
            }
            self.record(PlayQueueEventKind::AudioFileRemovedFromQueue {
                queue_id: self.id.clone(),
                audio_file_id: audio_file_id.clone(),
            });
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.current_index = None;
        self.record(PlayQueueEventKind::QueueCleared {
            queue_id: self.id.clone(),
        });
    }

    pub fn reorder(&mut self, audio_file_id: &AudioFileId, new_index: usize) {
        if let Some(old_index) = self.items.iter().position(|x| x == audio_file_id) {
            if old_index == new_index || new_index >= self.items.len() {
                return;
            }
            let item = self.items.remove(old_index);
            self.items.insert(new_index, item);
            // Adjust current_index for move operations across it
            if let Some(ci) = self.current_index {
                self.current_index = Some(adjust_index_after_move(ci, old_index, new_index));
            }
            self.record(PlayQueueEventKind::QueueReordered {
                queue_id: self.id.clone(),
            });
        }
    }

    pub fn set_current_index(&mut self, index: Option<usize>) {
        self.current_index = index.and_then(|i| (i < self.items.len()).then_some(i));
    }

    pub fn snapshot(&self) -> PlayQueueSnapshot {
        let (current, previous, next, current_index) = match self.current_index {
            Some(i) if i < self.items.len() => {
                let current = Some(self.items[i].clone());
                let previous = if i > 0 {
                    Some(self.items[i - 1].clone())
                } else {
                    None
                };
                let next = if i + 1 < self.items.len() {
                    Some(self.items[i + 1].clone())
                } else {
                    None
                };
                (current, previous, next, Some(i))
            }
            _ => (None, None, None, None),
        };
        PlayQueueSnapshot {
            queue_id: self.id.clone(),
            items: self.items.clone(),
            current,
            previous,
            next,
            current_index,
        }
    }

    pub fn take_pending_events(&mut self) -> Vec<PlayQueueDomainEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

#[derive(Debug, Clone)]
pub enum PlayQueueEventKind {
    AudioFileAddedToQueue {
        queue_id: PlayQueueId,
        audio_file_id: AudioFileId,
    },
    AudioFileRemovedFromQueue {
        queue_id: PlayQueueId,
        audio_file_id: AudioFileId,
    },
    QueueCleared {
        queue_id: PlayQueueId,
    },
    QueueReordered {
        queue_id: PlayQueueId,
    },
}

#[derive(Debug, Clone)]
pub struct PlayQueueDomainEvent {
    pub queue_id: PlayQueueId,
    pub kind: PlayQueueEventKind,
}

impl DomainEvent for PlayQueueDomainEvent {
    fn aggregate_id(&self) -> i64 {
        self.queue_id.as_i64()
    }
    fn version(&self) -> i64 {
        0
    }
}

#[derive(Debug, Clone)]
pub struct PlayQueueSnapshot {
    pub queue_id: PlayQueueId,
    pub items: Vec<AudioFileId>,
    pub current: Option<AudioFileId>,
    pub previous: Option<AudioFileId>,
    pub next: Option<AudioFileId>,
    pub current_index: Option<usize>,
}

fn adjust_index_after_move(current_index: usize, old_index: usize, new_index: usize) -> usize {
    // If the current index item stays the same, adjust index relative to the move
    if old_index < new_index {
        // Moving item forward shrinks indices in (old_index+1..=new_index)
        if (old_index + 1..=new_index).contains(&current_index) {
            return current_index - 1;
        }
    } else if new_index < old_index {
        // Moving item backward grows indices in (new_index..old_index)
        if (new_index..old_index).contains(&current_index) {
            return current_index + 1;
        }
    }
    current_index
}

#[async_trait]
pub trait PlayQueueRepository {
    async fn find_by_id(&self, id: PlayQueueId) -> Result<Option<PlayQueue>, PlayQueueError>;
    async fn find_by_user_id(&self, user_id: UserId) -> Result<Option<PlayQueue>, PlayQueueError>;
    async fn save(&self, play_queue: &mut PlayQueue) -> Result<(), PlayQueueError>;
    async fn delete(&self, id: PlayQueueId) -> Result<(), PlayQueueError>;
    async fn delete_by_user_id(&self, user_id: UserId) -> Result<(), PlayQueueError>;
}
