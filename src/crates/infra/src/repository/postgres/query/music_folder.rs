use super::db_data::music_folder as db_music_folder;
use application::query::dao::MusicFolderDao;
use application::query::QueryError;
use async_trait::async_trait;
use model::music_folder::MusicFolder;
use sea_orm::*;

pub struct MusicFolderDaoImpl {
    db: DatabaseConnection,
}

impl MusicFolderDaoImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MusicFolderDao for MusicFolderDaoImpl {
    async fn get_by_id(&self, id: i64) -> Result<Option<MusicFolder>, QueryError> {
        let folder: Option<db_music_folder::MusicFolderModel> =
            db_music_folder::MusicFolderModel::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"select id, name, last_scan_at from library where id = $1"#,
                vec![id.into()],
            ))
            .one(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;
        Ok(folder.map(|folder| folder.into()))
    }
    async fn get_all(&self) -> Result<Vec<MusicFolder>, QueryError> {
        let folders: Vec<db_music_folder::MusicFolderModel> =
            db_music_folder::MusicFolderModel::find_by_statement(Statement::from_string(
                DbBackend::Postgres,
                r#"select id, name, last_scan_at from library;
                "#,
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;
        Ok(folders.into_iter().map(|folder| folder.into()).collect())
    }
}
