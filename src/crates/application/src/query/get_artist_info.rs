use crate::query::dao::ArtistDao;
use crate::query::dto::artist_info::ArtistInfoDto;
use crate::query::dto::cover_art;
use crate::query::shared::CoverArtTokenService;
use crate::query::QueryError;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetArtistInfo {
    dao: Arc<dyn ArtistDao + Send + Sync>,
    token_service: Arc<dyn CoverArtTokenService>,
}

impl GetArtistInfo {
    pub fn new(
        dao: Arc<dyn ArtistDao + Send + Sync>,
        token_service: Arc<dyn CoverArtTokenService>,
    ) -> Self {
        Self { dao, token_service }
    }

    pub async fn handle(&self, artist_id: i64) -> Result<ArtistInfoDto, QueryError> {
        let artist_info = self
            .dao
            .get_artist_info(artist_id)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        let artist_info = match artist_info {
            Some(info) => info,
            None => {
                return Err(QueryError::InvalidInput(format!(
                    "Artist not found: {}",
                    artist_id
                )));
            }
        };

        let artist_cover_id = cover_art::artist_cover_art_id(artist_id);

        // 生成封面图片访问 token
        let cover_art_token = self
            .token_service
            .issue_cover_art_token(artist_cover_id)
            .map_err(|e| QueryError::ExecutionError(format!("Failed to generate token: {}", e)))?;

        Ok(ArtistInfoDto {
            artist_info,
            cover_art_token,
        })
    }
}
