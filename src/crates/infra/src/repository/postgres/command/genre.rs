use super::db_data::{genre, genre::ActiveModel, genre::Entity, genre::Model};
use async_trait::async_trait;
use domain::genre::{Genre, GenreError, GenreName, GenreRepository};
use domain::value::GenreId;
use sea_orm::entity::prelude::*;
use sea_orm::*;

#[derive(Clone)]
pub struct GenreRepositoryImpl {
    db: sea_orm::DbConn,
}

impl GenreRepositoryImpl {
    pub fn new(db: sea_orm::DbConn) -> Self {
        Self { db }
    }
}

#[async_trait]
impl GenreRepository for GenreRepositoryImpl {
    async fn find_by_id(&self, id: GenreId) -> Result<Option<Genre>, GenreError> {
        let row: Option<Model> = Entity::find_by_id(id.as_i64())
            .one(&self.db)
            .await
            .map_err(|e| GenreError::DbErr(e.to_string()))?;
        Ok(row.map(|m| m.into()))
    }

    async fn find_by_name(&self, genre_name: &GenreName) -> Result<Option<Genre>, GenreError> {
        let row: Option<Model> = Entity::find()
            .filter(genre::Column::Name.eq(genre_name.value()))
            .one(&self.db)
            .await
            .map_err(|e| GenreError::DbErr(e.to_string()))?;
        Ok(row.map(|m| m.into()))
    }

    async fn save(&self, mut genre_agg: Genre) -> Result<Genre, GenreError> {
        genre_agg.version += 1;
        let genre_agg_cloned = genre_agg.clone();
        // Decide insert vs update by existence to avoid version bootstrap mismatch
        let exists = Entity::find_by_id(genre_agg_cloned.id.as_i64())
            .one(&self.db)
            .await
            .map_err(|e| GenreError::DbErr(e.to_string()))?
            .is_some();

        let active_model: ActiveModel = genre_agg_cloned.into();

        if !exists {
            Entity::insert(active_model)
                .exec(&self.db)
                .await
                .map_err(|e| GenreError::DbErr(e.to_string()))?;
            return Ok(genre_agg);
        }

        let update_condition = Condition::all()
            .add(genre::Column::Id.eq(genre_agg.id.as_i64()))
            .add(genre::Column::Version.lt(genre_agg.version() + 1));

        let result = Entity::update_many()
            .set(active_model)
            .filter(update_condition)
            .exec(&self.db)
            .await
            .map_err(|e| GenreError::DbErr(e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(GenreError::VersionConflictErr(genre_agg.version()));
        }

        Ok(genre_agg)
    }

    async fn delete(&self, genre_id: GenreId) -> Result<(), GenreError> {
        Entity::delete_by_id(genre_id.as_i64())
            .exec(&self.db)
            .await
            .map_err(|e| GenreError::DbErr(e.to_string()))?;
        Ok(())
    }
}
