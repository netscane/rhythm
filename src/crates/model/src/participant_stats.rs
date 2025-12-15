use crate::ModelError;
use domain::value::{ArtistId, ParticipantRole};

#[derive(Clone)]
pub struct ParticipantStats {
    pub artist_id: ArtistId,
    pub role: ParticipantRole,
    pub duration: i64,
    pub size: i64,
    pub song_count: i32,
    pub album_count: i32,
}

use async_trait::async_trait;

#[async_trait]
pub trait ParticipantStatsRepository: Send + Sync {
    async fn adjust_participant_stats(&self, delta: ParticipantStats) -> Result<(), ModelError>;
}
