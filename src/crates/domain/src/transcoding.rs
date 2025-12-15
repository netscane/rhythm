use futures::stream::Stream;
use std::collections::HashMap;
use thiserror::Error;

use crate::value::TranscodingId;

#[derive(Error, Debug)]
pub enum TranscodingError {
    #[error("基础设施错误: {0}")]
    InfrastructureErr(String),
    #[error("验证错误: {0}")]
    ValidationErr(String),
    #[error("转码错误: {0}")]
    TranscodingErr(String),
    #[error("执行错误: {0}")]
    ExecutionErr(String),
    #[error("版本号冲突: 期望 {0}, 实际 {1}")]
    VersionConflictErr(i64, i64),
    #[error("未找到: {0}")]
    NotFoundErr(String),
}

#[derive(Debug, Clone)]
pub struct Transcoding {
    pub id: TranscodingId,
    pub name: String,
    pub target_format: String,
    pub command: String,
    pub default_bit_rate: i32,
    pub parameters: HashMap<String, String>,
    pub version: i64,
}

impl Transcoding {
    pub fn new(
        id: TranscodingId,
        name: String,
        target_format: String,
        command: String,
        default_bit_rate: i32,
    ) -> Self {
        Self {
            id,
            name,
            target_format,
            command,
            default_bit_rate,
            parameters: HashMap::new(),
            version: 0,
        }
    }

    pub fn add_parameter(&mut self, key: String, value: String) -> Result<(), TranscodingError> {
        self.parameters.insert(key, value);
        Ok(())
    }

    pub fn remove_parameter(&mut self, key: &str) -> Result<(), TranscodingError> {
        self.parameters.remove(key);
        Ok(())
    }

    pub fn update_name(&mut self, name: String) -> Result<(), TranscodingError> {
        if name.is_empty() {
            return Err(TranscodingError::ValidationErr("名称不能为空".to_string()));
        }
        self.name = name;
        Ok(())
    }

    pub fn update_command(&mut self, command: String) -> Result<(), TranscodingError> {
        if command.is_empty() {
            return Err(TranscodingError::ValidationErr("命令不能为空".to_string()));
        }
        self.command = command;
        Ok(())
    }

    pub fn update_default_bit_rate(&mut self, bit_rate: i32) -> Result<(), TranscodingError> {
        if bit_rate <= 0 {
            return Err(TranscodingError::ValidationErr(
                "比特率必须大于0".to_string(),
            ));
        }
        self.default_bit_rate = bit_rate;
        Ok(())
    }
}

#[async_trait::async_trait]
pub trait TranscodingStreamer: Send + Sync {
    async fn create_stream(
        &self,
        input_path: String,
        output_format: String,
        bit_rate: i32,
        additional_params: HashMap<String, String>,
    ) -> Result<
        Box<dyn Stream<Item = Result<Vec<u8>, TranscodingError>> + Unpin + Send>,
        TranscodingError,
    >;
}

#[async_trait::async_trait]
pub trait TranscodingRepository: Send + Sync {
    async fn save(&self, transcoding: &mut Transcoding) -> Result<(), TranscodingError>;
    async fn find_by_id(&self, id: TranscodingId) -> Result<Option<Transcoding>, TranscodingError>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Transcoding>, TranscodingError>;
    async fn find_by_target_format(
        &self,
        format: &str,
    ) -> Result<Option<Transcoding>, TranscodingError>;
    async fn find_all(&self) -> Result<Vec<Transcoding>, TranscodingError>;
    async fn delete(&self, id: TranscodingId) -> Result<(), TranscodingError>;
}
