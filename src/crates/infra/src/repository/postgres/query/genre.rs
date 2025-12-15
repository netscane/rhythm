use super::db_data::{
    genre_stats, genre_stats::ActiveModel, genre_stats::Entity, genre_stats::Model,
};
use async_trait::async_trait;
use model::genre::{GenreStats, GenreStatsRepository};
use model::ModelError;
use sea_orm::entity::prelude::*;

#[derive(Clone)]
pub struct GenreStatsRepositoryImpl {
    db: sea_orm::DbConn,
}

impl GenreStatsRepositoryImpl {
    pub fn new(db: sea_orm::DbConn) -> Self {
        Self { db }
    }

    pub async fn get_all(&self) -> Result<Vec<GenreStats>, ModelError> {
        let rows: Vec<Model> = Entity::find().all(&self.db).await.map_err(map_db_error)?;
        Ok(rows.into_iter().map(|m| m.into()).collect())
    }
}

// Helper function to map database errors
#[inline]
fn map_db_error(e: DbErr) -> ModelError {
    ModelError::ProjectionError(e.to_string())
}

#[async_trait]
impl GenreStatsRepository for GenreStatsRepositoryImpl {
    async fn adjust_stats(&self, entry: GenreStats) -> Result<(), ModelError> {
        use sea_orm::sea_query::{Expr, OnConflict};

        let genre_id_val = entry.genre_id.as_i64();
        let song_count_val = entry.song_count;
        let album_count_val = entry.album_count;

        // Single SQL operation: INSERT ... ON CONFLICT DO UPDATE
        // Updates all fields at once, values can be positive (increment) or negative (decrement)
        let active_model = ActiveModel {
            genre_id: sea_orm::Set(genre_id_val),
            song_count: sea_orm::Set(song_count_val),
            album_count: sea_orm::Set(album_count_val),
        };

        Entity::insert(active_model)
            .on_conflict(
                OnConflict::column(genre_stats::Column::GenreId)
                    .value(
                        genre_stats::Column::SongCount,
                        Expr::col((genre_stats::Entity, genre_stats::Column::SongCount)).add(song_count_val),
                    )
                    .value(
                        genre_stats::Column::AlbumCount,
                        Expr::col((genre_stats::Entity, genre_stats::Column::AlbumCount)).add(album_count_val),
                    )
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(map_db_error)?;

        Ok(())
    }
}

pub struct GenreDaoImpl {
    db: sea_orm::DbConn,
}

impl GenreDaoImpl {
    pub fn new(db: sea_orm::DbConn) -> Self {
        Self { db }
    }
}

use super::db_data::genre as db_genre;
use application::query::dao::GenreDao;
use application::query::QueryError;
use model::genre::Genre;
use sea_orm::*;
#[async_trait]
impl GenreDao for GenreDaoImpl {
    async fn get_all(&self) -> Result<Vec<Genre>, QueryError> {
        let rows: Vec<db_genre::GenreModel> =
            db_genre::GenreModel::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                r#"select genre.name, genre_stats.song_count, genre_stats.album_count from genre_stats join genre on genre_stats.genre_id = genre.id;
            "#,
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;
        Ok(rows.into_iter().map(|m| m.into()).collect())
    }
}
