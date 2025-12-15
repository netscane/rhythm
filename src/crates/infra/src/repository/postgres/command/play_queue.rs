use super::db_data::{
    play_queue::{self, ActiveModel, Entity, Model},
    play_queue_item::{self, ActiveModel as ItemActiveModel, Entity as ItemEntity, Model as ItemModel, PlayQueueItemInsert},
};
use async_trait::async_trait;
use domain::play_queue::{PlayQueue, PlayQueueError, PlayQueueRepository};
use domain::value::{AudioFileId, PlayQueueId, UserId};
use sea_orm::*;

#[derive(Clone)]
pub struct PlayQueueRepositoryImpl {
    db: DbConn,
}

impl PlayQueueRepositoryImpl {
    pub fn new(db: DbConn) -> Self {
        Self { db }
    }

    async fn load_items(&self, play_queue_id: i64) -> Result<Vec<AudioFileId>, PlayQueueError> {
        let items: Vec<ItemModel> = ItemEntity::find()
            .filter(play_queue_item::Column::PlayQueueId.eq(play_queue_id))
            .order_by_asc(play_queue_item::Column::Position)
            .all(&self.db)
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        Ok(items.into_iter().map(|m| AudioFileId::from(m.audio_file_id)).collect())
    }
}

#[async_trait]
impl PlayQueueRepository for PlayQueueRepositoryImpl {
    async fn find_by_id(&self, id: PlayQueueId) -> Result<Option<PlayQueue>, PlayQueueError> {
        let result: Option<Model> = Entity::find_by_id(id.as_i64())
            .one(&self.db)
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        match result {
            Some(model) => {
                let items = self.load_items(id.as_i64()).await?;
                Ok(Some(model.into_play_queue(items)))
            }
            None => Ok(None),
        }
    }

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Option<PlayQueue>, PlayQueueError> {
        let result: Option<Model> = Entity::find()
            .filter(play_queue::Column::UserId.eq(user_id.as_i64()))
            .one(&self.db)
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        match result {
            Some(model) => {
                let items = self.load_items(model.id).await?;
                Ok(Some(model.into_play_queue(items)))
            }
            None => Ok(None),
        }
    }

    async fn save(&self, play_queue: &mut PlayQueue) -> Result<(), PlayQueueError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        let play_queue_id = play_queue.id.as_i64();

        // Check if exists
        let exists = Entity::find_by_id(play_queue_id)
            .one(&txn)
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?
            .is_some();

        let active_model: ActiveModel = (&*play_queue).into();

        if exists {
            // Update
            active_model
                .update(&txn)
                .await
                .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;
        } else {
            // Insert
            active_model
                .insert(&txn)
                .await
                .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;
        }

        // Delete all existing items
        ItemEntity::delete_many()
            .filter(play_queue_item::Column::PlayQueueId.eq(play_queue_id))
            .exec(&txn)
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        // Insert new items
        if !play_queue.items.is_empty() {
            let items: Vec<ItemActiveModel> = play_queue
                .items
                .iter()
                .enumerate()
                .map(|(pos, audio_file_id)| {
                    PlayQueueItemInsert {
                        id: play_queue_id * 1000000 + pos as i64, // Simple ID generation
                        play_queue_id,
                        audio_file_id: audio_file_id.as_i64(),
                        position: pos as i32,
                    }
                    .into()
                })
                .collect();

            ItemEntity::insert_many(items)
                .exec(&txn)
                .await
                .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;
        }

        txn.commit()
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: PlayQueueId) -> Result<(), PlayQueueError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        // Delete items
        ItemEntity::delete_many()
            .filter(play_queue_item::Column::PlayQueueId.eq(id.as_i64()))
            .exec(&txn)
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        // Delete play queue
        Entity::delete_by_id(id.as_i64())
            .exec(&txn)
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        txn.commit()
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_user_id(&self, user_id: UserId) -> Result<(), PlayQueueError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        // Find the play queue for this user
        let result: Option<Model> = Entity::find()
            .filter(play_queue::Column::UserId.eq(user_id.as_i64()))
            .one(&txn)
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        if let Some(model) = result {
            // Delete items
            ItemEntity::delete_many()
                .filter(play_queue_item::Column::PlayQueueId.eq(model.id))
                .exec(&txn)
                .await
                .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

            // Delete play queue
            Entity::delete_by_id(model.id)
                .exec(&txn)
                .await
                .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;
        }

        txn.commit()
            .await
            .map_err(|e| PlayQueueError::DbErr(e.to_string()))?;

        Ok(())
    }
}
