use super::db_data::{transcoding::ActiveModel, transcoding::Column, transcoding::Entity, transcoding::Model};
use async_trait::async_trait;
use domain::transcoding::{Transcoding, TranscodingError, TranscodingRepository};
use domain::value::TranscodingId;
use sea_orm::*;

#[derive(Clone)]
pub struct TranscodingRepositoryImpl {
    db: sea_orm::DbConn,
}

impl TranscodingRepositoryImpl {
    pub fn new(db: sea_orm::DbConn) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TranscodingRepository for TranscodingRepositoryImpl {
    async fn save(&self, transcoding: &mut Transcoding) -> Result<(), TranscodingError> {
        let old_version = transcoding.version;
        let am: ActiveModel = ActiveModel::from(&*transcoding);

        // 检查是否存在
        let existing = Entity::find_by_id(transcoding.id.as_i64())
            .one(&self.db)
            .await
            .map_err(|e| TranscodingError::InfrastructureErr(e.to_string()))?;

        if existing.is_some() {
            // 更新 - 使用乐观锁
            let result = Entity::update(am)
                .filter(Column::Version.eq(old_version))
                .exec(&self.db)
                .await;

            match result {
                Ok(_) => {
                    transcoding.version = old_version + 1;
                    Ok(())
                }
                Err(DbErr::RecordNotUpdated) => Err(TranscodingError::VersionConflictErr(
                    old_version,
                    transcoding.version,
                )),
                Err(e) => Err(TranscodingError::InfrastructureErr(e.to_string())),
            }
        } else {
            // 插入
            am.insert(&self.db)
                .await
                .map_err(|e| TranscodingError::InfrastructureErr(e.to_string()))?;
            transcoding.version = old_version + 1;
            Ok(())
        }
    }

    async fn find_by_id(&self, id: TranscodingId) -> Result<Option<Transcoding>, TranscodingError> {
        let result_row: Option<Model> = Entity::find_by_id(id.as_i64())
            .one(&self.db)
            .await
            .map_err(|e| TranscodingError::InfrastructureErr(e.to_string()))?;
        Ok(result_row.map(|row| row.into()))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Transcoding>, TranscodingError> {
        let result_row: Option<Model> = Entity::find()
            .filter(Column::Name.eq(name))
            .one(&self.db)
            .await
            .map_err(|e| TranscodingError::InfrastructureErr(e.to_string()))?;
        Ok(result_row.map(|row| row.into()))
    }

    async fn find_by_target_format(
        &self,
        format: &str,
    ) -> Result<Option<Transcoding>, TranscodingError> {
        let result_row: Option<Model> = Entity::find()
            .filter(Column::TargetFormat.eq(format))
            .one(&self.db)
            .await
            .map_err(|e| TranscodingError::InfrastructureErr(e.to_string()))?;
        Ok(result_row.map(|row| row.into()))
    }

    async fn find_all(&self) -> Result<Vec<Transcoding>, TranscodingError> {
        let result_rows: Vec<Model> = Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| TranscodingError::InfrastructureErr(e.to_string()))?;
        Ok(result_rows.into_iter().map(|row| row.into()).collect())
    }

    async fn delete(&self, id: TranscodingId) -> Result<(), TranscodingError> {
        Entity::delete_by_id(id.as_i64())
            .exec(&self.db)
            .await
            .map_err(|e| TranscodingError::InfrastructureErr(e.to_string()))?;
        Ok(())
    }
}
