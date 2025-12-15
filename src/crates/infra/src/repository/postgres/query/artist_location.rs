use async_trait::async_trait;
use chrono::Utc;
use domain::value::{ArtistId, MediaPath};
use log::debug;
use model::artist_location::{ArtistLocation, ArtistLocationRepository};
use sea_orm::{
    sea_query::{Expr, OnConflict},
    ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter, Set,
};

use super::db_data::artist_location::{ActiveModel, Column, Entity};

pub struct MysqlArtistLocationRepository {
    db: DatabaseConnection,
}

impl MysqlArtistLocationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ArtistLocationRepository for MysqlArtistLocationRepository {
    async fn adjust_count(
        &self,
        entry: ArtistLocation,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "Adjust count for artist_id: {}, location: {:?}, total: {}",
            entry.artist_id, entry.location, entry.total
        );

        let artist_id_val = i64::from(entry.artist_id);
        let count_val = entry.total; // Can be positive or negative

        // Get current timestamp for update_time
        let now = Utc::now().naive_utc();

        // Single SQL operation: INSERT ... ON CONFLICT DO UPDATE
        // If not exists: INSERT with entry.count and current timestamp
        // If exists: UPDATE count = count + entry.count and update_time = NOW()
        let active_model = ActiveModel {
            id: NotSet,
            artist_id: Set(artist_id_val),
            location_protocol: Set(entry.location.protocol.clone()),
            location_path: Set(entry.location.path.clone()),
            total: Set(count_val),
            update_time: Set(Some(now)),
        };

        Entity::insert_many([active_model])
            .on_conflict(
                OnConflict::columns([
                    Column::ArtistId,
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
            .await?;

        Ok(())
    }
}
