use crate::event::DomainEvent;
use crate::play_queue::PlayQueueSnapshot;
use crate::value::{AudioFileId, PlayQueueId, PlayerId, UserId};
use async_trait::async_trait;
use chrono::{Local, NaiveDateTime};
use thiserror::Error;

/// Domain events emitted by `Player`.
#[derive(Debug, Clone)]
pub enum PlayerEventKind {
    PlayQueueChanged {
        play_queue_id: PlayQueueId,
    },
    PlaybackStarted {
        user_id: UserId,
        audio_file_id: AudioFileId,
    },
    PlaybackPaused,
    PlaybackResumed,
    PlaybackStopped {
        audio_file_id: AudioFileId,
    },
    PlaybackModeChanged {
        mode: PlaybackMode,
    },
}

/// Concrete event carrying metadata required by the `DomainEvent` trait.
#[derive(Debug, Clone)]
pub struct PlayerEvent {
    pub player_id: PlayerId,
    pub version: i64,
    pub kind: PlayerEventKind,
}

impl DomainEvent for PlayerEvent {
    fn aggregate_id(&self) -> i64 {
        self.player_id.as_i64()
    }
    fn version(&self) -> i64 {
        self.version
    }
}

#[derive(Error, Debug)]
pub enum PlayerError {
    #[error("Version conflict: expected {expected}, got {actual}")]
    VersionConflict { expected: i32, actual: i32 },
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
    #[error("{0}")]
    OtherErr(String),
}

/// Finite playback state for a `Player` aggregate.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerState {
    Idle,
    Playing,
    Paused,
    Stopped,
}

/// Music playback client aggregate representing a single logical player.
#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub user_id: UserId,
    pub user_agent: String,
    pub client: String,
    pub ip: String,
    pub last_seen: NaiveDateTime,
    pub transcoding_id: String,
    pub max_bit_rate: i32,
    pub report_real_path: bool,
    pub scrobble_enabled: bool,
    pub version: i32,
    pub last_op_time: NaiveDateTime,
    pub state: PlayerState,
    pub current_item: Option<AudioFileId>,
    pub play_queue_id: Option<PlayQueueId>,
    pub volume: u8,
    pub mode: PlaybackMode,
    pub pending_events: Vec<PlayerEvent>,
}

impl Player {
    /// Create a new player with default telemetry and stopped state.
    pub fn new(
        id: PlayerId,
        user_id: UserId,
        user_agent: String,
        client: String,
        ip: String,
    ) -> Self {
        Self {
            id,
            name: format!("{} [{}]", client, user_agent),
            user_id,
            user_agent,
            client,
            ip,
            version: 0,
            last_op_time: Local::now().naive_local(),
            last_seen: Local::now().naive_local(),
            transcoding_id: String::new(),
            max_bit_rate: 0,
            report_real_path: false,
            scrobble_enabled: false,
            state: PlayerState::Stopped,
            current_item: None,
            play_queue_id: None,
            volume: 100,
            mode: PlaybackMode::Sequential,
            pending_events: Vec::new(),
        }
    }

    /// Update last seen timestamp without bumping version.
    fn touch_seen(&mut self) {
        self.last_seen = Local::now().naive_local();
    }

    /// Update last op time and optionally bump optimistic concurrency version.
    fn touch_op(&mut self, bump_version: bool) {
        self.last_op_time = Local::now().naive_local();
        if bump_version {
            self.version = self.version.saturating_add(1);
        }
    }

    /// Emit and buffer a domain event for later dispatch.
    fn record(&mut self, kind: PlayerEventKind) {
        let event = PlayerEvent {
            player_id: self.id.clone(),
            version: self.version as i64,
            kind,
        };
        self.pending_events.push(event);
    }

    /// Lightweight ping from client to refresh presence.
    pub fn heartbeat(&mut self) {
        self.touch_seen();
    }

    /// Update UA/client/IP telemetry. Bumps version.
    pub fn update_client_info(&mut self, user_agent: String, client: String, ip: String) {
        self.user_agent = user_agent;
        self.client = client;
        self.ip = ip;
        self.touch_op(true);
    }

    /// Configure maximum output bitrate. Bumps version.
    pub fn set_max_bit_rate(&mut self, max_bit_rate: i32) {
        self.max_bit_rate = max_bit_rate;
        self.touch_op(true);
    }

    /// Toggle reporting of real filesystem paths. Bumps version.
    pub fn set_report_real_path(&mut self, enabled: bool) {
        self.report_real_path = enabled;
        self.touch_op(true);
    }

    /// Enable scrobbling behavior. Bumps version.
    pub fn enable_scrobble(&mut self) {
        self.scrobble_enabled = true;
        self.touch_op(true);
    }

    /// Disable scrobbling behavior. Bumps version.
    pub fn disable_scrobble(&mut self) {
        self.scrobble_enabled = false;
        self.touch_op(true);
    }

    /// Associate current transcoding session. Bumps version.
    pub fn set_transcoding_id(&mut self, transcoding_id: String) {
        self.transcoding_id = transcoding_id;
        self.touch_op(true);
    }

    /// Clear transcoding association. Bumps version.
    pub fn clear_transcoding(&mut self) {
        self.transcoding_id.clear();
        self.touch_op(true);
    }

    /// Start playback of a song at the given position (ms). Emits `PlaybackStarted`.
    pub fn play(&mut self, item_id: AudioFileId) -> Result<(), PlayerError> {
        match self.state {
            PlayerState::Stopped | PlayerState::Idle => {
                self.current_item = Some(item_id.clone());
                self.state = PlayerState::Playing;
                self.touch_seen();
                self.touch_op(true);
                self.record(PlayerEventKind::PlaybackStarted {
                    user_id: self.user_id.clone(),
                    audio_file_id: item_id,
                });
                Ok(())
            }
            PlayerState::Playing => {
                // If already playing same item, treat as seek
                if self.current_item == Some(item_id.clone()) {
                    self.touch_seen();
                    self.touch_op(true);
                    Ok(())
                } else {
                    if let Some(old) = self.current_item.clone() {
                        self.record(PlayerEventKind::PlaybackStopped { audio_file_id: old });
                    }
                    self.current_item = Some(item_id.clone());
                    self.touch_seen();
                    self.touch_op(true);
                    self.record(PlayerEventKind::PlaybackStarted {
                        user_id: self.user_id.clone(),
                        audio_file_id: item_id,
                    });
                    Ok(())
                }
            }
            PlayerState::Paused => {
                // Resume with possibly new item
                if let Some(old) = self.current_item.clone() {
                    if old != item_id {
                        self.record(PlayerEventKind::PlaybackStopped { audio_file_id: old });
                    }
                }
                self.current_item = Some(item_id.clone());
                self.state = PlayerState::Playing;
                self.touch_seen();
                self.touch_op(true);
                self.record(PlayerEventKind::PlaybackStarted {
                    user_id: self.user_id.clone(),
                    audio_file_id: item_id,
                });
                Ok(())
            }
        }
    }

    /// Pause playback. Only valid from Playing. Emits `PlaybackPaused`.
    pub fn pause(&mut self) -> Result<(), PlayerError> {
        match self.state {
            PlayerState::Playing => {
                self.state = PlayerState::Paused;
                self.touch_seen();
                self.touch_op(true);
                self.record(PlayerEventKind::PlaybackPaused);
                Ok(())
            }
            _ => Err(PlayerError::InvalidStateTransition {
                from: format!("{:?}", self.state),
                to: "Paused".to_string(),
            }),
        }
    }

    /// Resume playback. Only valid from Paused. Emits `PlaybackResumed`.
    pub fn resume(&mut self) -> Result<(), PlayerError> {
        match self.state {
            PlayerState::Paused => {
                self.state = PlayerState::Playing;
                self.touch_seen();
                self.touch_op(true);
                self.record(PlayerEventKind::PlaybackResumed);
                Ok(())
            }
            _ => Err(PlayerError::InvalidStateTransition {
                from: format!("{:?}", self.state),
                to: "Playing".to_string(),
            }),
        }
    }

    /// Stop playback and clear current item. Emits `PlaybackStopped`.
    pub fn stop(&mut self) -> Result<(), PlayerError> {
        match self.state {
            PlayerState::Playing | PlayerState::Paused => {
                if let Some(old) = self.current_item.clone() {
                    self.record(PlayerEventKind::PlaybackStopped { audio_file_id: old });
                }
                self.state = PlayerState::Stopped;
                self.current_item = None;
                self.touch_seen();
                self.touch_op(true);
                Ok(())
            }
            PlayerState::Stopped | PlayerState::Idle => Ok(()),
        }
    }

    /// Set playback mode. Emits `PlaybackModeChanged`.
    pub fn set_mode(&mut self, mode: PlaybackMode) {
        self.mode = mode.clone();
        self.touch_op(true);
        self.record(PlayerEventKind::PlaybackModeChanged { mode });
    }

    /// Set or clear the associated play queue. Emits `PlayQueueChanged` when set.
    pub fn set_play_queue_id(&mut self, play_queue_id: Option<PlayQueueId>) {
        self.play_queue_id = play_queue_id.clone();
        self.touch_op(true);
        if let Some(id) = play_queue_id {
            self.record(PlayerEventKind::PlayQueueChanged { play_queue_id: id });
        }
    }

    pub fn pop_events(&mut self) -> Vec<PlayerEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Compute the next song according to current `PlaybackMode` using a queue snapshot.
    pub fn get_next_song(&self, snapshot: &PlayQueueSnapshot) -> Option<AudioFileId> {
        match self.mode {
            PlaybackMode::Sequential => snapshot.next.clone(),
            PlaybackMode::RepeatOne => snapshot.current.clone(),
            PlaybackMode::RepeatAll => snapshot
                .next
                .clone()
                .or_else(|| snapshot.items.first().cloned()),
            PlaybackMode::Shuffle => self.pick_shuffle(&snapshot.items, snapshot.current.as_ref()),
        }
    }

    /// Compute the previous song according to current `PlaybackMode` using a queue snapshot.
    pub fn get_previous_song(&self, snapshot: &PlayQueueSnapshot) -> Option<AudioFileId> {
        match self.mode {
            PlaybackMode::Sequential => snapshot.previous.clone(),
            PlaybackMode::RepeatOne => snapshot.current.clone(),
            PlaybackMode::RepeatAll => snapshot
                .previous
                .clone()
                .or_else(|| snapshot.items.last().cloned()),
            PlaybackMode::Shuffle => self.pick_shuffle(&snapshot.items, snapshot.current.as_ref()),
        }
    }

    /// Deterministic shuffle without external RNG; avoids immediately repeating the same song when possible.
    fn pick_shuffle(
        &self,
        items: &Vec<AudioFileId>,
        current: Option<&AudioFileId>,
    ) -> Option<AudioFileId> {
        if items.is_empty() {
            return None;
        }
        if items.len() == 1 {
            return items.first().cloned();
        }
        let secs = self.last_op_time.and_utc().timestamp() as i64;
        let nanos = self.last_op_time.and_utc().timestamp_subsec_nanos() as i64;
        let seed = (secs ^ (nanos << 32)) ^ (self.version as i64);
        let mut idx = (seed % (items.len() as i64)) as usize;
        if let Some(cur) = current {
            if items.get(idx) == Some(cur) {
                idx = (idx + 1) % items.len();
            }
        }
        items.get(idx).cloned()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackMode {
    Sequential,
    Shuffle,
    RepeatOne,
    RepeatAll,
}

#[async_trait]
pub trait PlayerRepository {
    async fn find_by_id(&self, id: PlayerId) -> Result<Option<Player>, PlayerError>;
    async fn save(&self, player: &mut Player) -> Result<(), PlayerError>;
    async fn delete(&self, id: PlayerId) -> Result<(), PlayerError>;
}
