use super::artwork_id::ArtworkId;
use model::album::Album;
use model::artist::Artist;

#[derive(Debug)]
pub struct ArtistIndex {
    pub id: String,
    pub artists: Vec<Artist>,
}

/// 带 token 的艺术家信息
#[derive(Debug)]
pub struct ArtistWithToken {
    pub artist: Artist,
    pub cover_art_id: String,
    /// 封面图片访问 token（基于 cover_art_id）
    pub cover_art_token: String,
}

impl ArtistWithToken {
    /// 从艺术家创建带 token 的 DTO
    /// 自动生成 cover_art_id 格式：ar-{artist_id}
    pub fn from_artist_with_token_service(
        artist: Artist,
        token_service: &dyn crate::query::shared::CoverArtTokenService,
    ) -> Option<Self> {
        // 生成 artwork_id
        let artwork_id = ArtworkId::for_artist(artist.id);
        let cover_art_id = artwork_id.to_string();
        let cover_art_token = token_service.issue_cover_art_token(cover_art_id.clone()).ok()?;
        
        Some(Self {
            artist,
            cover_art_id,
            cover_art_token,
        })
    }

    /// 从艺术家创建带 token 的 DTO（使用 ArtworkId）
    /// 支持带更新时间的 cover_art_id
    /// 
    /// 注意：此方法已弃用，请使用 `from_artist_with_token_service` 代替
    #[deprecated(since = "0.1.0", note = "use from_artist_with_token_service instead")]
    pub fn from_artist_with_artwork_id(
        artist: Artist,
        token_service: &dyn crate::query::shared::CoverArtTokenService,
    ) -> Option<Self> {
        Self::from_artist_with_token_service(artist, token_service)
    }
}

/// 带 token 的艺术家索引
#[derive(Debug)]
pub struct ArtistIndexWithTokens {
    pub id: String,
    pub artists: Vec<ArtistWithToken>,
}

pub struct ArtistWithAlbumDto {
    pub artist: Artist,
    pub albums: Vec<Album>,
    pub cover_art_token: String,
}
