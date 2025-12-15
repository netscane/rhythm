use crate::value::{PlaylistId, UserId};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use thiserror::Error;

/// 播放列表领域错误
#[derive(Error, Debug)]
pub enum PlaylistError {
    #[error("Database error: {0}")]
    DbErr(String),
    #[error("Repository error: {0}")]
    RepositoryErr(String),
    #[error("Validation error: {0}")]
    ValidationErr(String),
    #[error("Entry not found: {0}")]
    EntryNotFound(i64),
    #[error("{0}")]
    OtherErr(String),
}

/// 播放列表所有者值对象
#[derive(Debug, Clone, PartialEq)]
pub struct Owner {
    pub id: UserId,
    pub name: String,
}

impl Default for Owner {
    fn default() -> Self {
        Self {
            id: UserId::from(0),
            name: String::new(),
        }
    }
}

/// 播放列表条目实体
#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistEntry {
    pub id: i64,
    pub playlist_id: PlaylistId,
    pub audio_file_id: i64,
    pub position: i32,
    pub added_at: NaiveDateTime,
}

impl PlaylistEntry {
    fn new(id: i64, playlist_id: PlaylistId, audio_file_id: i64, position: i32) -> Self {
        Self {
            id,
            playlist_id,
            audio_file_id,
            position,
            added_at: Utc::now().naive_utc(),
        }
    }
}

/// 播放列表聚合根
#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: String,
    pub comment: Option<String>,
    pub owner: Owner,
    pub public: bool,
    pub entries: Vec<PlaylistEntry>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub version: i64,
    pub deleted: bool,
}

impl Playlist {
    /// 创建新播放列表
    pub fn new(
        id: PlaylistId,
        name: &str,
        owner: Owner,
        comment: Option<&str>,
        public: bool,
    ) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            id,
            name: name.to_string(),
            comment: comment.map(|s| s.to_string()),
            owner,
            public,
            entries: Vec::new(),
            created_at: now,
            updated_at: now,
            version: 0,
            deleted: false,
        }
    }

    /// 添加条目
    pub fn add_entry(&mut self, entry_id: i64, audio_file_id: i64) {
        let position = self.entries.len() as i32;
        let entry = PlaylistEntry::new(entry_id, self.id.clone(), audio_file_id, position);
        self.entries.push(entry);
        self.touch();
    }

    /// 删除条目
    pub fn remove_entry(&mut self, entry_id: i64) -> Result<(), PlaylistError> {
        let idx = self
            .entries
            .iter()
            .position(|e| e.id == entry_id)
            .ok_or(PlaylistError::EntryNotFound(entry_id))?;

        self.entries.remove(idx);
        self.touch();
        Ok(())
    }

    /// 更新名称
    pub fn update_name(&mut self, name: &str) {
        self.name = name.to_string();
        self.touch();
    }

    /// 更新备注
    pub fn update_comment(&mut self, comment: Option<&str>) {
        self.comment = comment.map(|s| s.to_string());
        self.touch();
    }

    /// 设置公开状态
    pub fn set_public(&mut self, public: bool) {
        self.public = public;
        self.touch();
    }

    /// 标记删除
    pub fn delete(&mut self) {
        self.deleted = true;
        self.touch();
    }

    /// 是否已删除
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    /// 获取活跃条目数量
    pub fn song_count(&self) -> usize {
        self.entries.len()
    }

    /// 获取条目迭代器
    pub fn entries(&self) -> impl Iterator<Item = &PlaylistEntry> {
        self.entries.iter()
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now().naive_utc();
        self.version += 1;
    }
}

/// 播放列表仓储接口
#[async_trait]
pub trait PlaylistRepository: Send + Sync {
    /// 根据 ID 查找
    async fn find_by_id(&self, id: PlaylistId) -> Result<Option<Playlist>, PlaylistError>;

    /// 保存播放列表
    async fn save(&self, playlist: &mut Playlist) -> Result<(), PlaylistError>;

    /// 删除播放列表
    async fn delete(&self, id: PlaylistId) -> Result<(), PlaylistError>;

    /// 清空所有数据
    async fn truncate(&self) -> Result<(), PlaylistError>;

    /// 根据所有者 ID 查找
    async fn find_by_owner_id(&self, owner_id: UserId) -> Result<Vec<Playlist>, PlaylistError>;
}
