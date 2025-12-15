use crate::query::dao::AlbumDao;
use crate::query::QueryError;
use model::album::Album;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetAlbumList {
    album_dao: Arc<dyn AlbumDao + Send + Sync>,
}

impl GetAlbumList {
    pub fn new(album_dao: Arc<dyn AlbumDao + Send + Sync>) -> Self {
        Self { album_dao }
    }

    pub async fn handle(
        &self,
        typ: &str,
        genre: Option<&str>,
        from_year: Option<i32>,
        to_year: Option<i32>,
        offset: i32,
        size: i32,
    ) -> Result<(Vec<Album>, i64), QueryError> {
        // 对于需要 user_id 的类型（starred），使用默认 user_id = 0
        // 这是为了向后兼容，新代码应该使用 handle_with_user
        self.handle_with_user(typ, genre, from_year, to_year, offset, size, 0).await
    }

    pub async fn handle_with_user(
        &self,
        typ: &str,
        genre: Option<&str>,
        from_year: Option<i32>,
        to_year: Option<i32>,
        offset: i32,
        size: i32,
        user_id: i64,
    ) -> Result<(Vec<Album>, i64), QueryError> {
        let limit = size.min(500); // 限制最大值为 500

        match typ {
            "newest" => self.album_dao.get_by_newest(offset, limit).await,
            "recent" => self.album_dao.get_by_recent(offset, limit).await,
            "random" => self.album_dao.get_by_random(offset, limit).await,
            "alphabeticalByName" => self.album_dao.get_by_name(offset, limit).await,
            "alphabeticalByArtist" => self.album_dao.get_by_artist(offset, limit).await,
            "frequent" => self.album_dao.get_by_frequent(offset, limit).await,
            "starred" => self.album_dao.get_by_starred(user_id, offset, limit).await,
            "highest" => self.album_dao.get_by_rating(offset, limit).await,
            "byGenre" => {
                let genre = genre.ok_or_else(|| {
                    QueryError::InvalidInput(
                        "genre parameter is required for byGenre type".to_string(),
                    )
                })?;
                self.album_dao.get_by_genre(genre, offset, limit).await
            }
            "byYear" => {
                let from_year = from_year.ok_or_else(|| {
                    QueryError::InvalidInput(
                        "fromYear parameter is required for byYear type".to_string(),
                    )
                })?;
                let to_year = to_year.ok_or_else(|| {
                    QueryError::InvalidInput(
                        "toYear parameter is required for byYear type".to_string(),
                    )
                })?;
                self.album_dao
                    .get_by_year(from_year, to_year, offset, limit)
                    .await
            }
            _ => Err(QueryError::InvalidInput(format!(
                "type '{}' not implemented",
                typ
            ))),
        }
    }
}
