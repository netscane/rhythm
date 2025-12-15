use crate::query::dao::{AlbumDao, ArtistDao};
use crate::query::dto::artist::ArtistWithAlbumDto;
use crate::query::dto::cover_art;
use crate::query::shared::CoverArtTokenService;
use crate::query::QueryError;
use model::album::Album;
use model::artist::Artist;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetArtist {
    artist_dao: Arc<dyn ArtistDao + Send + Sync>,
    album_dao: Arc<dyn AlbumDao + Send + Sync>,
    token_service: Arc<dyn CoverArtTokenService>,
}

impl GetArtist {
    pub fn new(
        artist_dao: Arc<dyn ArtistDao + Send + Sync>,
        album_dao: Arc<dyn AlbumDao + Send + Sync>,
        token_service: Arc<dyn CoverArtTokenService>,
    ) -> Self {
        Self {
            artist_dao,
            album_dao,
            token_service,
        }
    }

    pub async fn handle(&self, artist_id: i64) -> Result<ArtistWithAlbumDto, QueryError> {
        let artist = self
            .artist_dao
            .get_by_id(artist_id)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        let artist = match artist {
            Some(artist) => artist,
            None => {
                return Err(QueryError::InvalidInput(format!(
                    "Artist not found: {}",
                    artist_id
                )));
            }
        };

        let albums = self
            .album_dao
            .get_by_artist_id(artist_id)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;
        
        let artist_cover_id = cover_art::artist_cover_art_id(artist_id);

        // 生成封面图片访问 token
        let cover_art_token = self
            .token_service
            .issue_cover_art_token(artist_cover_id)
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        Ok(ArtistWithAlbumDto {
            artist,
            albums,
            cover_art_token,
        })
    }
}
