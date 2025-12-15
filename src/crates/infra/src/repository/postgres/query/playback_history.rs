use super::db_data::playback_history as db;
use async_trait::async_trait;
use domain::value::UserId;
use model::playback_history::{PlaybackHistoryEntry, PlaybackHistoryRepository};
use model::ModelError;
use sea_orm::*;

pub struct PlaybackHistoryRepositoryImpl {
    db: DatabaseConnection,
}

impl PlaybackHistoryRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    async fn get_recent_by_user(
        &self,
        user_id: &UserId,
        limit: usize,
    ) -> Result<Vec<PlaybackHistoryEntry>, ModelError> {
        let rows = db::Entity::find()
            .filter(db::Column::UserId.eq(user_id.as_i64()))
            .order_by_desc(db::Column::ScrobbledAt)
            .limit(limit as u64)
            .all(&self.db)
            .await
            .map_err(|e| ModelError::ProjectionError(e.to_string()))?;
        Ok(rows.into_iter().map(PlaybackHistoryEntry::from).collect())
    }
}

#[async_trait]
impl PlaybackHistoryRepository for PlaybackHistoryRepositoryImpl {
    async fn save(&self, entry: &PlaybackHistoryEntry) -> Result<(), ModelError> {
        let model: db::ActiveModel = entry.into();
        model
            .insert(&self.db)
            .await
            .map_err(|e| ModelError::ProjectionError(e.to_string()))?;
        Ok(())
    }
}
