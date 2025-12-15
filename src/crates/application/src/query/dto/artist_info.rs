use model::artist::ArtistInfo;

/// 艺术家信息 DTO，包含封面图片访问 token
#[derive(Debug)]
pub struct ArtistInfoDto {
    pub artist_info: ArtistInfo,
    /// 封面图片访问 token（用于生成安全的图片 URL）
    pub cover_art_token: String,
}

