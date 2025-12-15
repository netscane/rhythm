use super::db_data::album_location as entry_db;
use async_trait::async_trait;
use chrono::Utc;
use domain::value::{AlbumId, MediaPath};
use model::album_location::{AlbumLocation, AlbumLocationRepository};
use model::ModelError;
use sea_orm::*;

// Helper function to map database errors
#[inline]
fn map_db_error(e: DbErr) -> ModelError {
    ModelError::ProjectionError(e.to_string())
}

pub struct MysqlAlbumLocationRepository {
    db: DatabaseConnection,
}

impl MysqlAlbumLocationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AlbumLocationRepository for MysqlAlbumLocationRepository {
    async fn adjust_count(&self, entry: AlbumLocation) -> Result<(), ModelError> {
        use entry_db::{Column, Entity};
        use sea_orm::sea_query::{Expr, OnConflict};

        let album_id_val = i64::from(entry.album_id);
        let count_val = entry.total; // Can be positive or negative

        // Get current timestamp for update_time
        let now = Utc::now().naive_utc();

        // Single SQL operation: INSERT ... ON CONFLICT DO UPDATE
        // If not exists: INSERT with entry.total and current timestamp
        // If exists: UPDATE count = count + entry.total and update_time = NOW()
        let active_model = entry_db::ActiveModel {
            id: NotSet,
            album_id: Set(album_id_val),
            location_protocol: Set(entry.location.protocol.clone()),
            location_path: Set(entry.location.path.clone()),
            total: Set(count_val),
            update_time: Set(Some(now)),
        };

        Entity::insert_many([active_model])
            .on_conflict(
                OnConflict::columns([
                    Column::AlbumId,
                    Column::LocationProtocol,
                    Column::LocationPath,
                ])
                .value(
                    Column::Total,
                    Expr::col((Entity, Column::Total)).add(count_val),
                )
                .value(Column::UpdateTime, Expr::current_timestamp())
                .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(map_db_error)?;

        Ok(())
    }
}
