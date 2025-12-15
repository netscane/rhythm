use super::db_data::system_config::{ActiveModel, Column, Entity};
use application::shared::SystemConfigStore;
use async_trait::async_trait;
use sea_orm::*;

#[derive(Clone)]
pub struct SystemConfigStoreImpl {
    db: DatabaseConnection,
}

impl SystemConfigStoreImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SystemConfigStore for SystemConfigStoreImpl {
    async fn get_string(&self, key: &str) -> anyhow::Result<Option<String>> {
        let result = Entity::find_by_id(key.to_string())
            .one(&self.db)
            .await?;
        Ok(result.map(|m| m.value))
    }

    async fn set_string(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let now = chrono::Utc::now().naive_utc();
        let active_model = ActiveModel {
            key: Set(key.to_string()),
            value: Set(value.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };

        Entity::insert(active_model)
            .on_conflict(
                sea_query::OnConflict::column(Column::Key)
                    .update_columns([Column::Value, Column::UpdatedAt])
                    .to_owned(),
            )
            .exec(&self.db)
            .await?;

        Ok(())
    }
}
