use model::album::AlbumInfo;

/// 专辑信息 DTO，包含封面图片访问 token
#[derive(Debug)]
pub struct AlbumInfoDto {
    pub album_info: AlbumInfo,
    /// 封面图片访问 token（用于生成安全的图片 URL）
    pub cover_art_token: String,
}
