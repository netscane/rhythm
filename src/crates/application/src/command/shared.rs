use crate::error::AppError;

/// 通用ID生成器接口，所有需要生成唯一ID的领域服务都可以使用此接口
#[async_trait::async_trait]
pub trait IdGenerator: Send + Sync {
    /// 生成下一个唯一ID
    async fn next_id(&self) -> Result<i64, AppError>;

    /// 根据业务标识生成下一个唯一ID
    /// business_key 可以是不同的业务类型标识，如 "genre"、"album" 等
    async fn next_id_with_business(&self, business_key: &str) -> Result<i64, AppError>;
}
