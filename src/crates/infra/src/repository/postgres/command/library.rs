use async_trait::async_trait;
use domain::library::{Library, LibraryError, LibraryItem, LibraryRepository};
use domain::value::LibraryId;
use sea_orm::*;
use std::collections::HashSet;

use super::db_data::{
    library::ActiveModel, library::Column as LibraryColumn, library::Entity as LibraryEntity,
    library_item::ActiveModel as ItemActiveModel, library_item::Column as ItemColumn,
    library_item::Entity as ItemEntity,
};

#[derive(Clone)]
pub struct LibraryRepositoryImpl {
    db: sea_orm::DbConn,
}

impl LibraryRepositoryImpl {
    pub fn new(db: sea_orm::DbConn) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LibraryRepository for LibraryRepositoryImpl {
    async fn save(&self, library: &Library) -> Result<(), LibraryError> {
        let mut library = library.clone();
        library.version += 1;

        let library = library.clone();
        self.db
            .transaction::<_, _, LibraryError>(|txn| {
                Box::pin(async move {
                    // 仅在本事务内关闭同步提交，加速提交延迟
                    txn
                        .execute(Statement::from_string(
                            DatabaseBackend::Postgres,
                            "SET LOCAL synchronous_commit = OFF".to_owned(),
                        ))
                        .await
                        .map_err(|e| LibraryError::DbError(e.to_string()))?;

                    // 获取现有的 items
                    let existing_items = ItemEntity::find()
                        .filter(ItemColumn::LibraryId.eq(library.id.as_i64()))
                        .all(txn)
                        .await
                        .map_err(|e| LibraryError::DbError(e.to_string()))?;

                    // 计算差异
                    let existing_ids: HashSet<i64> = existing_items.iter().map(|i| i.id).collect();
                    let new_ids: HashSet<i64> = library
                        .items
                        .iter()
                        .map(|(_, item)| item.id.as_i64())
                        .collect();

                    // 需要删除的 items
                    let ids_to_delete: Vec<i64> =
                        existing_ids.difference(&new_ids).cloned().collect();

                    // 需要新增的 items
                    let items_to_insert: Vec<ItemActiveModel> = library
                        .items
                        .iter()
                        .filter(|(_, item)| !existing_ids.contains(&item.id.as_i64()))
                        .map(|(_, item)| item.clone().into())
                        .collect();

                    // 更新库基本信息(带版本控制)
                    let active_model: ActiveModel = library.clone().into();
                    let update_condition = Condition::all()
                        .add(LibraryColumn::Id.eq(library.id.as_i64()))
                        .add(LibraryColumn::Version.lt(library.version + 1));

                    let result = LibraryEntity::update_many()
                        .set(active_model)
                        .filter(update_condition)
                        .exec(txn)
                        .await
                        .map_err(|e| LibraryError::DbError(e.to_string()))?;

                    if result.rows_affected == 0 {
                        return Err(LibraryError::DbError("版本号冲突".to_string()));
                    }

                    // 批量删除不再需要的 items
                    if !ids_to_delete.is_empty() {
                        ItemEntity::delete_many()
                            .filter(ItemColumn::Id.is_in(ids_to_delete))
                            .filter(ItemColumn::LibraryId.eq(library.id.as_i64()))
                            .exec(txn)
                            .await
                            .map_err(|e| LibraryError::DbError(e.to_string()))?;
                    }

                    // 批量插入新的 items,每批100条
                    if !items_to_insert.is_empty() {
                        for chunk in items_to_insert.chunks(30) {
                            ItemEntity::insert_many(chunk.to_vec())
                                .exec(txn)
                                .await
                                .map_err(|e| LibraryError::DbError(e.to_string()))?;
                        }
                    }

                    Ok(())
                })
            })
            .await
            .map_err(|e| LibraryError::DbError(e.to_string()))
    }

    async fn find_by_id(&self, id: &LibraryId) -> Result<Option<Library>, LibraryError> {
        let model_opt = LibraryEntity::find_by_id(id.as_i64())
            .one(&self.db)
            .await
            .map_err(|e| LibraryError::DbError(e.to_string()))?;

        let Some(model) = model_opt else {
            return Ok(None);
        };

        let mut items = Vec::new();
        let mut offset = 0;
        let page_size = 1000;
        loop {
            let batch: Vec<LibraryItem> = ItemEntity::find()
                .filter(ItemColumn::LibraryId.eq(id.as_i64()))
                .limit(page_size as u64)
                .offset(offset as u64)
                .all(&self.db)
                .await
                .map_err(|e| LibraryError::DbError(e.to_string()))?
                .into_iter()
                .map(|m| m.into())
                .collect();
            let batch_len = batch.len();
            items.extend(batch);
            if batch_len < page_size {
                break;
            }
            offset += page_size;
        }

        let mut library: Library = model.into();
        library.items = items
            .into_iter()
            .map(|i| (i.path.path.clone(), i))
            .collect();

        Ok(Some(library))
    }
}
