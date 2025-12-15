use super::db_data::{annotation, annotation::ActiveModel, annotation::Entity, annotation::Model};
use async_trait::async_trait;
use domain::annotation::{Annotation, AnnotationError, AnnotationRepository, Kind};
use domain::value::UserId;
use sea_orm::*;

#[derive(Clone)]
pub struct AnnotationRepositoryImpl {
    db: sea_orm::DbConn,
}

impl AnnotationRepositoryImpl {
    pub fn new(db: sea_orm::DbConn) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AnnotationRepository for AnnotationRepositoryImpl {
    async fn find_by_item(
        &self,
        item_kind: Kind,
        item_id: i64,
    ) -> Result<Option<Annotation>, AnnotationError> {
        let result_row: Option<Model> = Entity::find()
            .filter(annotation::Column::ItemId.eq(item_id))
            .filter(annotation::Column::ItemKind.eq(item_kind.to_string()))
            .one(&self.db)
            .await
            .map_err(|e| AnnotationError::DbErr(e.to_string()))?;
        if let Some(row) = result_row {
            Ok(Some(row.into()))
        } else {
            Ok(None)
        }
    }

    async fn find_by_item_id(&self, item_id: i64) -> Result<Option<Annotation>, AnnotationError> {
        let result_row: Option<Model> = Entity::find()
            .filter(annotation::Column::ItemId.eq(item_id))
            .one(&self.db)
            .await
            .map_err(|e| AnnotationError::DbErr(e.to_string()))?;
        if let Some(row) = result_row {
            Ok(Some(row.into()))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, annotation: Annotation) -> Result<(), AnnotationError> {
        self.db
            .transaction::<_, (), AnnotationError>(move |txn| {
                let annotation = annotation.clone();
                Box::pin(async move {
                    // 先查询记录是否存在
                    let existing: Option<Model> = Entity::find_by_id(annotation.id.as_i64())
                        .one(txn)
                        .await
                        .map_err(|e| AnnotationError::DbErr(e.to_string()))?;

                    match existing {
                        None => {
                            // 记录不存在，执行插入
                            let active_model: ActiveModel = annotation.into();
                            active_model
                                .insert(txn)
                                .await
                                .map_err(|e| AnnotationError::DbErr(e.to_string()))?;
                        }
                        Some(existing_model) => {
                            // 记录存在，检查版本号
                            if annotation.version <= existing_model.version {
                                return Err(AnnotationError::InvalidOperation(
                                    "版本号必须大于当前版本号".to_string(),
                                ));
                            }

                            // 执行更新，使用乐观锁
                            let mut update_model: ActiveModel = annotation.clone().into();
                            update_model.created_at = NotSet;
                            let update_condition = Condition::all()
                                .add(annotation::Column::Id.eq(annotation.id.as_i64()))
                                .add(annotation::Column::Version.eq(existing_model.version));

                            let result = Entity::update_many()
                                .set(update_model)
                                .filter(update_condition)
                                .exec(txn)
                                .await
                                .map_err(|e| AnnotationError::DbErr(e.to_string()))?;

                            // 验证乐观锁
                            if result.rows_affected == 0 {
                                return Err(AnnotationError::InvalidOperation(
                                    "版本号冲突，数据已被其他事务修改".to_string(),
                                ));
                            }
                        }
                    }
                    Ok(())
                })
            })
            .await
            .map_err(|e| AnnotationError::DbErr(e.to_string()))?;
        Ok(())
    }

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Vec<Annotation>, AnnotationError> {
        let rows: Vec<Model> = Entity::find()
            .filter(annotation::Column::UserId.eq(user_id.as_i64()))
            .all(&self.db)
            .await
            .map_err(|e| AnnotationError::DbErr(e.to_string()))?;
        let v = rows.into_iter().map(|row| row.into()).collect();
        Ok(v)
    }

    async fn delete(&self, annotation: Annotation) -> Result<(), AnnotationError> {
        Entity::delete_by_id(annotation.id.as_i64())
            .exec(&self.db)
            .await
            .map_err(|e| AnnotationError::DbErr(e.to_string()))?;
        Ok(())
    }
    async fn delete_all(&self) -> Result<(), AnnotationError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AnnotationError::DbErr(e.to_string()))?;

        Entity::delete_many()
            .exec(&txn)
            .await
            .map_err(|e| AnnotationError::DbErr(e.to_string()))?;

        txn.commit()
            .await
            .map_err(|e| AnnotationError::DbErr(e.to_string()))?;
        Ok(())
    }
}
