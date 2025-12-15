use super::db_data::{
    playlist::{self, ActiveModel, Entity, Model},
    playlist_entry::{self, ActiveModel as EntryActiveModel, Entity as EntryEntity, Model as EntryModel},
};
use async_trait::async_trait;
use domain::playlist::{Playlist, PlaylistEntry, PlaylistError, PlaylistRepository};
use domain::value::{PlaylistId, UserId};
use sea_orm::*;
use std::collections::HashSet;

#[derive(Clone)]
pub struct PlaylistRepositoryImpl {
    db: DbConn,
}

impl PlaylistRepositoryImpl {
    pub fn new(db: DbConn) -> Self {
        Self { db }
    }

    async fn load_entries(&self, playlist_id: i64) -> Result<Vec<PlaylistEntry>, PlaylistError> {
        let entries: Vec<EntryModel> = EntryEntity::find()
            .filter(playlist_entry::Column::PlaylistId.eq(playlist_id))
            .order_by_asc(playlist_entry::Column::Position)
            .all(&self.db)
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        Ok(entries.into_iter().map(|m| m.into()).collect())
    }
}

#[async_trait]
impl PlaylistRepository for PlaylistRepositoryImpl {
    async fn find_by_id(&self, id: PlaylistId) -> Result<Option<Playlist>, PlaylistError> {
        let result: Option<Model> = Entity::find_by_id(id.as_i64())
            .one(&self.db)
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        match result {
            Some(model) => {
                let mut playlist: Playlist = model.into();
                playlist.entries = self.load_entries(id.as_i64()).await?;
                Ok(Some(playlist))
            }
            None => Ok(None),
        }
    }

    async fn save(&self, playlist: &mut Playlist) -> Result<(), PlaylistError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        let playlist_id = playlist.id.as_i64();

        if playlist.is_deleted() {
            // 删除条目
            EntryEntity::delete_many()
                .filter(playlist_entry::Column::PlaylistId.eq(playlist_id))
                .exec(&txn)
                .await
                .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

            // 删除播放列表
            Entity::delete_by_id(playlist_id)
                .exec(&txn)
                .await
                .map_err(|e| PlaylistError::DbErr(e.to_string()))?;
        } else {
            // 检查是否存在
            let exists = Entity::find_by_id(playlist_id)
                .one(&txn)
                .await
                .map_err(|e| PlaylistError::DbErr(e.to_string()))?
                .is_some();

            let active_model: ActiveModel = (&*playlist).into();

            if exists {
                // 更新
                active_model
                    .update(&txn)
                    .await
                    .map_err(|e| PlaylistError::DbErr(e.to_string()))?;
            } else {
                // 插入
                active_model
                    .insert(&txn)
                    .await
                    .map_err(|e| PlaylistError::DbErr(e.to_string()))?;
            }

            // 加载现有条目 ID
            let existing_entries: Vec<EntryModel> = EntryEntity::find()
                .filter(playlist_entry::Column::PlaylistId.eq(playlist_id))
                .all(&txn)
                .await
                .map_err(|e| PlaylistError::DbErr(e.to_string()))?;
            let existing_ids: HashSet<i64> = existing_entries.iter().map(|e| e.id).collect();

            // 计算新条目 ID
            let new_ids: HashSet<i64> = playlist.entries.iter().map(|e| e.id).collect();

            // 删除缺失的条目
            let to_delete: Vec<i64> = existing_ids.difference(&new_ids).copied().collect();
            if !to_delete.is_empty() {
                EntryEntity::delete_many()
                    .filter(playlist_entry::Column::Id.is_in(to_delete))
                    .exec(&txn)
                    .await
                    .map_err(|e| PlaylistError::DbErr(e.to_string()))?;
            }

            // 插入新条目
            let to_insert: Vec<EntryActiveModel> = playlist
                .entries
                .iter()
                .filter(|e| !existing_ids.contains(&e.id))
                .map(|e| e.into())
                .collect();
            if !to_insert.is_empty() {
                EntryEntity::insert_many(to_insert)
                    .exec(&txn)
                    .await
                    .map_err(|e| PlaylistError::DbErr(e.to_string()))?;
            }
        }

        txn.commit()
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: PlaylistId) -> Result<(), PlaylistError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        // 删除条目
        EntryEntity::delete_many()
            .filter(playlist_entry::Column::PlaylistId.eq(id.as_i64()))
            .exec(&txn)
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        // 删除播放列表
        Entity::delete_by_id(id.as_i64())
            .exec(&txn)
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        txn.commit()
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        Ok(())
    }

    async fn truncate(&self) -> Result<(), PlaylistError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        EntryEntity::delete_many()
            .exec(&txn)
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        Entity::delete_many()
            .exec(&txn)
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        txn.commit()
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        Ok(())
    }

    async fn find_by_owner_id(&self, owner_id: UserId) -> Result<Vec<Playlist>, PlaylistError> {
        let playlists: Vec<Model> = Entity::find()
            .filter(playlist::Column::OwnerId.eq(owner_id.as_i64()))
            .all(&self.db)
            .await
            .map_err(|e| PlaylistError::DbErr(e.to_string()))?;

        let mut result = Vec::new();
        for model in playlists {
            let playlist_id = model.id;
            let mut playlist: Playlist = model.into();
            playlist.entries = self.load_entries(playlist_id).await?;
            result.push(playlist);
        }

        Ok(result)
    }
}
