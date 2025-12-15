use crate::query::dao::{AlbumDao, ArtistDao, AudioFileDao};
use crate::query::dto::artist::ArtistWithToken;
use crate::query::shared::CoverArtTokenService;
use crate::query::QueryError;
use model::album::Album;
use model::artist::Artist;
use model::audio_file::AudioFile;
use std::sync::Arc;

#[derive(Clone)]
pub struct GetStarred {
    artist_dao: Arc<dyn ArtistDao + Send + Sync>,
    album_dao: Arc<dyn AlbumDao + Send + Sync>,
    audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
    token_service: Option<Arc<dyn CoverArtTokenService>>,
}

impl GetStarred {
    pub fn new(
        artist_dao: Arc<dyn ArtistDao + Send + Sync>,
        album_dao: Arc<dyn AlbumDao + Send + Sync>,
        audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
    ) -> Self {
        Self {
            artist_dao,
            album_dao,
            audio_file_dao,
            token_service: None,
        }
    }

    pub fn with_token_service(
        artist_dao: Arc<dyn ArtistDao + Send + Sync>,
        album_dao: Arc<dyn AlbumDao + Send + Sync>,
        audio_file_dao: Arc<dyn AudioFileDao + Send + Sync>,
        token_service: Arc<dyn CoverArtTokenService>,
    ) -> Self {
        Self {
            artist_dao,
            album_dao,
            audio_file_dao,
            token_service: Some(token_service),
        }
    }

    /// 获取收藏的内容（不带 token）
    pub async fn handle(&self, user_id: i64) -> Result<(Vec<Artist>, Vec<Album>, Vec<AudioFile>), QueryError> {
        let artists = self.artist_dao.get_by_starred(user_id).await?;
        let albums = self.album_dao.get_starred(user_id).await?;
        let audio_files = self.audio_file_dao.get_by_starred(user_id).await?;

        Ok((artists, albums, audio_files))
    }

    /// 获取收藏的内容（带 token）
    pub async fn handle_with_tokens(
        &self,
        user_id: i64,
    ) -> Result<(Vec<ArtistWithToken>, Vec<Album>, Vec<AudioFile>), QueryError> {
        let token_service = self.token_service.as_ref().ok_or_else(|| {
            QueryError::InvalidInput("Token service not configured".to_string())
        })?;

        let artists = self.artist_dao.get_by_starred(user_id).await?;
        let albums = self.album_dao.get_starred(user_id).await?;
        let audio_files = self.audio_file_dao.get_by_starred(user_id).await?;

        // 为艺术家生成 cover_art_id 和 token
        let artists_with_tokens: Vec<ArtistWithToken> = artists
            .into_iter()
            .filter_map(|artist| {
                ArtistWithToken::from_artist_with_token_service(artist, token_service.as_ref())
            })
            .collect();

        Ok((artists_with_tokens, albums, audio_files))
    }
}
