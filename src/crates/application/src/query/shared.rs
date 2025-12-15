use crate::error::AppError;

/// Token 生成服务 trait（用于生成封面图片访问 token）
/// 应用服务层通过此 trait 生成 token，不依赖具体实现
pub trait CoverArtTokenService: Send + Sync {
    /// 为指定的封面图片 ID 生成访问 token
    fn issue_cover_art_token(&self, cover_art_id: String) -> Result<String, AppError>;
}
