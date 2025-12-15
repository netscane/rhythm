use crate::event::DomainEvent;
use crate::value::GenreId;
use thiserror::Error;

// 领域错误定义
#[derive(Debug, Error)]
pub enum GenreError {
    #[error("数据库错误: {0}")]
    DbErr(String),
    #[error("实体不存在: {0}")]
    NotFoundErr(String),
    #[error("版本冲突: {0}")]
    VersionConflictErr(i64),
    #[error("验证错误: {0}")]
    ValidationErr(String),
    #[error("关联错误: {0}")]
    RelationError(String),
    #[error("其他错误: {0}")]
    OtherErr(String),
}

#[derive(Debug, Clone)]
pub enum GenreEvent {
    Created(GenreCreated),
    Found(GenreFound),
}

impl DomainEvent for GenreEvent {
    fn aggregate_id(&self) -> i64 {
        match self {
            GenreEvent::Created(event) => event.genre_id.as_i64(),
            GenreEvent::Found(event) => event.genre_id.as_i64(),
        }
    }
    fn version(&self) -> i64 {
        match self {
            GenreEvent::Created(event) => event.version,
            GenreEvent::Found(event) => event.version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenreCreated {
    pub genre_id: GenreId,
    pub version: i64,
}

#[derive(Debug, Clone)]
pub struct GenreFound {
    pub genre_id: GenreId,
    pub version: i64,
}

// 流派值对象 - 保持名称验证
#[derive(Debug, Clone, PartialEq)]
pub struct GenreName(String);

impl GenreName {
    pub fn new(name: String) -> Result<Self, GenreError> {
        if name.trim().is_empty() {
            return Err(GenreError::ValidationErr("流派名称不能为空".to_string()));
        }

        if name.len() > 50 {
            return Err(GenreError::ValidationErr(
                "流派名称不能超过50个字符".to_string(),
            ));
        }
        Ok(Self(name))
    }

    pub fn value(&self) -> String {
        self.0.clone()
    }
}

// 流派实体 - 领域根
#[derive(Debug, Clone)]
pub struct Genre {
    pub id: GenreId,
    pub name: GenreName,
    pub version: i64,
    pending_events: Vec<GenreEvent>,
}

impl Genre {
    pub fn new(id: GenreId, genre_name: GenreName) -> Result<Self, GenreError> {
        let mut genre = Self {
            id,
            name: genre_name,
            version: 0,
            pending_events: Vec::new(),
        };
        genre.pending_events.push(GenreEvent::Created(GenreCreated {
            genre_id: genre.id.clone(),
            version: 0,
        }));
        Ok(genre)
    }
    pub fn with_version(self, version: i64) -> Self {
        Self { version, ..self }
    }

    pub fn name(&self) -> String {
        self.name.value()
    }

    pub fn version(&self) -> i64 {
        self.version
    }

    pub fn take_events(&mut self) -> Vec<GenreEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
// 仓储接口 - 依赖反转
#[async_trait::async_trait]
pub trait GenreRepository: Send + Sync {
    // 查询方法只保留必要的find_by_name用于检查重复，以及find_by_id用于实现命令操作
    async fn find_by_id(&self, id: GenreId) -> Result<Option<Genre>, GenreError>;

    async fn find_by_name(&self, genre_name: &GenreName) -> Result<Option<Genre>, GenreError>;

    async fn save(&self, mut genre: Genre) -> Result<Genre, GenreError>;

    async fn delete(&self, genre_id: GenreId) -> Result<(), GenreError>;
}
