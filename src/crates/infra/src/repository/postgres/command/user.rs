use super::db_data::user::{self, ActiveModel, Column, Entity, Model};
use async_trait::async_trait;
use domain::user::{User, UserError};
use domain::value::UserId;
use sea_orm::*;

#[derive(Clone)]
pub struct UserRepositoryImpl {
    db: DatabaseConnection,
}

#[async_trait]
impl domain::user::UserRepository for UserRepositoryImpl {
    async fn count(&self) -> Result<u64, UserError> {
        let count = user::Entity::find()
            .count(&self.db)
            .await
            .map_err(|e| UserError::DbErr(e.to_string()))?;
        Ok(count)
    }

    async fn find_by_username<'a>(&'a self, username: &'a str) -> Result<Option<User>, UserError> {
        let result = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| UserError::DbErr(e.to_string()))?;
        match result {
            Some(model) => Ok(Some(model.into())),
            None => Err(UserError::InvalidUserOrPassword(username.to_owned())),
        }
    }

    async fn find_by_id<'a>(&'a self, id: UserId) -> Result<Option<User>, UserError> {
        let result = user::Entity::find_by_id(id.as_i64())
            .one(&self.db)
            .await
            .map_err(|e| UserError::DbErr(e.to_string()))?;
        Ok(result.map(|model| model.into()))
    }

    async fn save<'a>(&'a self, agg: &User) -> Result<(), UserError> {
        let mut active_model: ActiveModel = agg.clone().into();
        // select by id
        let existing = user::Entity::find_by_id(agg.id.as_i64())
            .one(&self.db)
            .await
            .map_err(|e| UserError::DbErr(e.to_string()))?;
        if existing.is_some() {
            active_model.version = Set(existing.unwrap().version + 1);
            let update_condition = Condition::all()
                .add(user::Column::Id.eq(agg.id.as_i64()))
                .add(user::Column::Version.lt(agg.version + 1));
            let result = Entity::update_many()
                .set(active_model)
                .filter(update_condition)
                .exec(&self.db)
                .await
                .map_err(|e| UserError::DbErr(e.to_string()))?;
            if result.rows_affected == 0 {
                return Err(UserError::VersionConflictErr(agg.version));
            }
            return Ok(());
        } else {
            active_model.version = Set(1);
            Entity::insert(active_model)
                .exec(&self.db)
                .await
                .map_err(|e| UserError::DbErr(e.to_string()))?;
            return Ok(());
        }

        Ok(())
    }

    async fn delete<'a>(&'a self, username: &'a str) -> Result<(), UserError> {
        let result = Entity::delete_many()
            .filter(Column::Username.eq(username))
            .exec(&self.db)
            .await
            .map_err(|e| UserError::DbErr(e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(UserError::UserNotFound(username.to_string()));
        }

        Ok(())
    }
}

impl UserRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    pub async fn find_by_username<'a>(
        &'a self,
        username: &'a str,
    ) -> Result<Option<User>, UserError> {
        let result = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| UserError::DbErr(e.to_string()))?;
        Ok(result.map(|model| model.into()))
    }
}
