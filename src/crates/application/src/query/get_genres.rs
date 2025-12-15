use crate::query::dao::GenreDao;
use crate::query::QueryError;
use model::genre::Genre;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetGenres {
    dao: Arc<dyn GenreDao + Send + Sync>,
}

impl GetGenres {
    pub fn new(dao: Arc<dyn GenreDao + Send + Sync>) -> Self {
        Self { dao }
    }

    pub async fn handle(&self) -> Result<Vec<Genre>, QueryError> {
        let genres = self
            .dao
            .get_all()
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;
        Ok(genres)
    }
}

