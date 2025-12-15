#[async_trait::async_trait]
pub trait SystemConfigStore: Send + Sync {
    async fn get_string(&self, key: &str) -> anyhow::Result<Option<String>>;
    async fn set_string(&self, key: &str, value: &str) -> anyhow::Result<()>;

    /// 如果不存在则设置（常用于 first_time 或默认配置）
    async fn get_or_set_default(&self, key: &str, default: &str) -> anyhow::Result<String> {
        if let Some(v) = self.get_string(key).await? {
            Ok(v)
        } else {
            self.set_string(key, default).await?;
            Ok(default.to_string())
        }
    }
}
