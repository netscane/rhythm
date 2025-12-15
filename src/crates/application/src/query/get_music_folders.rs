use crate::query::dao::MusicFolderDao;
use crate::query::QueryError;
use model::music_folder::MusicFolder;
use std::sync::Arc;
#[derive(Clone)]
pub struct GetMusicFolders {
    dao: Arc<dyn MusicFolderDao + Send + Sync>,
}

impl GetMusicFolders {
    pub fn new(dao: Arc<dyn MusicFolderDao + Send + Sync>) -> Self {
        Self { dao }
    }

    pub async fn handle(&self) -> Result<Vec<MusicFolder>, QueryError> {
        let folders = self
            .dao
            .get_all()
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;
        Ok(folders)
    }
}
