use super::db_data::participant_stats as stat_db;
use async_trait::async_trait;
use model::participant_stats::{ParticipantStats, ParticipantStatsRepository};
use model::ModelError;
use sea_orm::*;

pub struct MysqlParticipantStatsRepository {
    db: DatabaseConnection,
}

impl MysqlParticipantStatsRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// Helper function to map database errors
#[inline]
fn map_db_error(e: DbErr) -> ModelError {
    ModelError::ProjectionError(e.to_string())
}

#[async_trait]
impl ParticipantStatsRepository for MysqlParticipantStatsRepository {
    async fn adjust_participant_stats(&self, entry: ParticipantStats) -> Result<(), ModelError> {
        use sea_orm::sea_query::{Expr, OnConflict};

        let artist_id_val = entry.artist_id.as_i64();
        let role_str = entry.role.to_string();
        let duration_val = entry.duration;
        let size_val = entry.size as i64;
        let song_count_val = entry.song_count;
        let album_count_val = entry.album_count;

        // Single SQL operation: INSERT ... ON CONFLICT DO UPDATE (all fields at once)
        // If not exists: INSERT with entry values
        // If exists: UPDATE by adding entry values to existing values
        let active_model = stat_db::ActiveModel {
            id: sea_orm::NotSet, // Let database auto-generate the ID
            artist_id: sea_orm::Set(artist_id_val),
            role: sea_orm::Set(role_str),
            duration: sea_orm::Set(duration_val),
            size: sea_orm::Set(size_val),
            song_count: sea_orm::Set(song_count_val),
            album_count: sea_orm::Set(album_count_val),
        };

        stat_db::Entity::insert(active_model)
            .on_conflict(
                OnConflict::columns([stat_db::Column::ArtistId, stat_db::Column::Role])
                    .value(stat_db::Column::Duration, Expr::col((stat_db::Entity, stat_db::Column::Duration)).add(duration_val))
                    .value(stat_db::Column::Size, Expr::col((stat_db::Entity, stat_db::Column::Size)).add(size_val))
                    .value(stat_db::Column::SongCount, Expr::col((stat_db::Entity, stat_db::Column::SongCount)).add(song_count_val))
                    .value(stat_db::Column::AlbumCount, Expr::col((stat_db::Entity, stat_db::Column::AlbumCount)).add(album_count_val))
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(map_db_error)?;

        Ok(())
    }
}
