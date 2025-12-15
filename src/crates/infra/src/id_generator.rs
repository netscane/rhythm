use application::command::shared::IdGenerator;
use application::error::AppError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const NODE_ID_BITS: i64 = 10;
const SEQUENCE_BITS: i64 = 12;
const MAX_NODE_ID: i64 = (1 << NODE_ID_BITS) - 1;
const MAX_SEQUENCE: i64 = (1 << SEQUENCE_BITS) - 1;
const TIMESTAMP_SHIFT: i64 = NODE_ID_BITS + SEQUENCE_BITS;
const NODE_ID_SHIFT: i64 = SEQUENCE_BITS;
const EPOCH: i64 = 1609459200000; // 2021-01-01 00:00:00 UTC

/// 雪花算法ID生成器
pub struct SnowflakeIdGenerator {
    node_id: i64,
    last_timestamp: Arc<Mutex<i64>>,
    sequence: Arc<Mutex<i64>>,
    business_map: Arc<Mutex<HashMap<String, (i64, i64)>>>, // 业务类型 -> (上一个时间戳, 序列号)
}

impl SnowflakeIdGenerator {
    /// 创建新的雪花算法ID生成器
    pub fn new(node_id: i64) -> Result<Self, AppError> {
        if node_id > MAX_NODE_ID {
            return Err(AppError::UnknownError(format!(
                "节点ID不能超过{}",
                MAX_NODE_ID
            )));
        }

        Ok(Self {
            node_id,
            last_timestamp: Arc::new(Mutex::new(0)),
            sequence: Arc::new(Mutex::new(0)),
            business_map: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// 获取当前时间戳（毫秒）
    fn get_timestamp() -> Result<i64, AppError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .map_err(|e| AppError::UnknownError(format!("获取系统时间失败: {}", e)))
    }

    /// 生成雪花算法ID
    fn generate_id(&self, timestamp: i64, node_id: i64, sequence: i64) -> i64 {
        ((timestamp - EPOCH) << TIMESTAMP_SHIFT) | (node_id << NODE_ID_SHIFT) | sequence
    }

    /// 等待下一个毫秒
    async fn wait_next_millis(&self, last_timestamp: i64) -> Result<i64, AppError> {
        let mut timestamp = Self::get_timestamp()?;
        while timestamp <= last_timestamp {
            tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
            timestamp = Self::get_timestamp()?;
        }
        Ok(timestamp)
    }
}

#[async_trait]
impl IdGenerator for SnowflakeIdGenerator {
    async fn next_id(&self) -> Result<i64, AppError> {
        let mut last_timestamp = self.last_timestamp.lock().await;
        let mut sequence = self.sequence.lock().await;

        let mut timestamp = Self::get_timestamp()?;

        if timestamp < *last_timestamp {
            return Err(AppError::UnknownError(
                "系统时钟回拨，拒绝生成ID".to_string(),
            ));
        }

        if timestamp == *last_timestamp {
            *sequence = (*sequence + 1) & MAX_SEQUENCE;
            if *sequence == 0 {
                timestamp = self.wait_next_millis(*last_timestamp).await?;
            }
        } else {
            *sequence = 0;
        }

        *last_timestamp = timestamp;
        Ok(self.generate_id(timestamp, self.node_id, *sequence))
    }

    async fn next_id_with_business(&self, business_key: &str) -> Result<i64, AppError> {
        let mut business_map = self.business_map.lock().await;
        let mut timestamp = Self::get_timestamp()?;
        let (mut last_timestamp, mut sequence) =
            business_map.get(business_key).copied().unwrap_or((0, 0));

        if timestamp < last_timestamp {
            return Err(AppError::UnknownError(
                "系统时钟回拨，拒绝生成ID".to_string(),
            ));
        }

        if timestamp == last_timestamp {
            sequence = (sequence + 1) & MAX_SEQUENCE;
            if sequence == 0 {
                drop(business_map); // 释放锁，避免在异步等待过程中持有锁
                timestamp = self.wait_next_millis(last_timestamp).await?;
                business_map = self.business_map.lock().await;
            }
        } else {
            sequence = 0;
        }

        business_map.insert(business_key.to_string(), (timestamp, sequence));
        Ok(self.generate_id(timestamp, self.node_id, sequence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tokio::runtime::Runtime;

    #[test]
    fn test_snowflake_id_generator() {
        let rt = Runtime::new().unwrap();
        let generator = SnowflakeIdGenerator::new(1).unwrap();

        // 测试生成多个ID，确保唯一性
        let mut ids = HashSet::new();
        for _ in 0..1000 {
            let id = rt.block_on(generator.next_id()).unwrap();
            assert!(!ids.contains(&id), "ID重复: {}", id);
            ids.insert(id);
        }
    }

    #[test]
    fn test_business_specific_id_generator() {
        let rt = Runtime::new().unwrap();
        let generator = SnowflakeIdGenerator::new(1).unwrap();

        // 测试不同业务类型生成的ID
        rt.block_on(generator.next_id_with_business("genre"))
            .unwrap();
        let genre_id = rt
            .block_on(generator.next_id_with_business("genre"))
            .unwrap();
        let album_id = rt
            .block_on(generator.next_id_with_business("album"))
            .unwrap();

        // 测试相同业务类型生成的ID唯一性
        let mut genre_ids = HashSet::new();
        for _ in 0..100 {
            let id = rt
                .block_on(generator.next_id_with_business("genre"))
                .unwrap();
            assert!(!genre_ids.contains(&id), "业务ID重复: {}", id);
            genre_ids.insert(id);
        }
    }
}
