use super::db_data::{player, player::ActiveModel, player::Entity, player::Model};
use async_trait::async_trait;
use domain::player::{Player, PlayerError, PlayerRepository};
use domain::value::PlayerId;
use sea_orm::*;

#[derive(Clone)]
pub struct PlayerRepositoryImpl {
    db: sea_orm::DbConn,
}

impl PlayerRepositoryImpl {
    pub fn new(db: sea_orm::DbConn) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PlayerRepository for PlayerRepositoryImpl {
    async fn find_by_id(&self, id: PlayerId) -> Result<Option<Player>, PlayerError> {
        let result_row: Option<Model> = Entity::find_by_id(i64::from(id))
            .one(&self.db)
            .await
            .map_err(|e| PlayerError::OtherErr(e.to_string()))?;
        if let Some(row) = result_row {
            Ok(Some(row.into()))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, player: &mut Player) -> Result<(), PlayerError> {
        let mut active_model: ActiveModel = player.clone().into();
        let existing = player::Entity::find_by_id(player.id.as_i64())
            .one(&self.db)
            .await
            .map_err(|e| PlayerError::OtherErr(e.to_string()))?;
        if let Some(existing_model) = existing {
            let existing_version = existing_model.version;
            active_model.version = Set(existing_version + 1);
            active_model.created_at = NotSet;
            let update_condition = Condition::all()
                .add(player::Column::Id.eq(player.id.as_i64()))
                .add(player::Column::Version.lt(existing_version + 1));
            let result = Entity::update_many()
                .set(active_model)
                .filter(update_condition)
                .exec(&self.db)
                .await
                .map_err(|e| PlayerError::OtherErr(e.to_string()))?;
            if result.rows_affected == 0 {
                return Err(PlayerError::VersionConflict {
                    expected: player.version,
                    actual: existing_version,
                });
            }
            return Ok(());
        } else {
            active_model.version = Set(1);
            Entity::insert(active_model)
                .exec(&self.db)
                .await
                .map_err(|e| PlayerError::OtherErr(e.to_string()))?;
            return Ok(());
        }
    }

    async fn delete(&self, id: PlayerId) -> Result<(), PlayerError> {
        Entity::delete_by_id(i64::from(id))
            .exec(&self.db)
            .await
            .map_err(|e| PlayerError::OtherErr(e.to_string()))?;
        Ok(())
    }
}
