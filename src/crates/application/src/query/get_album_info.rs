use crate::query::dao::AlbumDao;
use crate::query::dto::album_info::AlbumInfoDto;
use crate::query::dto::cover_art;
use crate::query::shared::CoverArtTokenService;
use crate::query::QueryError;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetAlbumInfo {
    dao: Arc<dyn AlbumDao + Send + Sync>,
    token_service: Arc<dyn CoverArtTokenService>,
}

impl GetAlbumInfo {
    pub fn new(
        dao: Arc<dyn AlbumDao + Send + Sync>,
        token_service: Arc<dyn CoverArtTokenService>,
    ) -> Self {
        Self { dao, token_service }
    }

    pub async fn handle(&self, album_id: i64) -> Result<AlbumInfoDto, QueryError> {
        let album_info = self
            .dao
            .get_album_info(album_id)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        let album_info = match album_info {
            Some(info) => info,
            None => {
                return Err(QueryError::InvalidInput(format!(
                    "Album not found: {}",
                    album_id
                )));
            }
        };

        let album_cover_id = cover_art::album_cover_art_id(album_id);

        // 生成封面图片访问 token
        let cover_art_token = self
            .token_service
            .issue_cover_art_token(album_cover_id)
            .map_err(|e| QueryError::ExecutionError(format!("Failed to generate token: {}", e)))?;

        Ok(AlbumInfoDto {
            album_info,
            cover_art_token,
        })
    }
}
