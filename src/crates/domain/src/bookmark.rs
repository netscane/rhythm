use super::id::{IdError, IdGenerator};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::future::Future;
use std::marker::Unpin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
// 领域错误定义
#[derive(Error, Debug)]
pub enum BookmarkError {
    #[error("Invalid bookmark position: {0}")]
    InvalidPosition(i64),
    #[error("Bookmark not found: {0}")]
    NotFound(i64),
    #[error("Concurrent modification detected")]
    ConcurrencyConflict,
    #[error("Repository error: {0}")]
    RepositoryError(String),
    #[error("ID generation error: {0}")]
    IdGenerationError(#[from] IdError),
    #[error("Domain error: {0}")]
    DomainError(String),
    #[error("Song not found: {0}")]
    SongNotFound(i64),
}

pub type Result<T> = std::result::Result<T, BookmarkError>;

// 定义实体状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityStatus {
    New,     // 新创建，尚未保存
    Active,  // 正常活跃状态
    Deleted, // 已标记删除
}

impl Default for EntityStatus {
    fn default() -> Self {
        EntityStatus::New
    }
}

// Bookmark 实体
#[derive(Debug, Clone)]
pub struct Bookmark {
    pub id: i64,
    pub song_id: i64,
    pub comment: String,
    pub position: i64,
    pub changed_by: i64,
    pub version: i64,
    pub old_version: i64,
    pub status: EntityStatus,
}

impl Default for Bookmark {
    fn default() -> Self {
        Self {
            id: 0,
            song_id: 0,
            comment: String::new(),
            position: 0,
            changed_by: 0,
            version: 0,
            old_version: 0,
            status: EntityStatus::New,
        }
    }
}

impl Bookmark {
    // 创建新的书签
    pub fn new(song_id: i64, position: i64, comment: String, user_id: i64) -> Result<Self> {
        if position < 0 {
            return Err(BookmarkError::InvalidPosition(position));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Ok(Self {
            id: 0, // 将由Repository分配
            song_id,
            comment,
            position,
            changed_by: user_id,
            version: 1,
            old_version: 1,
            status: EntityStatus::New,
        })
    }

    // 更新书签位置
    pub fn update_position(&mut self, position: i64, user_id: i64) -> Result<()> {
        if self.status == EntityStatus::Deleted {
            return Err(BookmarkError::DomainError(
                "Cannot modify deleted bookmark".to_string(),
            ));
        }

        if position < 0 {
            return Err(BookmarkError::InvalidPosition(position));
        }

        self.position = position;
        self.changed_by = user_id;
        self.update_timestamp_and_version();
        Ok(())
    }

    // 更新评论
    pub fn update_comment(&mut self, comment: String, user_id: i64) -> Result<()> {
        if self.status == EntityStatus::Deleted {
            return Err(BookmarkError::DomainError(
                "Cannot modify deleted bookmark".to_string(),
            ));
        }

        self.comment = comment;
        self.changed_by = user_id;
        self.update_timestamp_and_version();
        Ok(())
    }

    // 标记为删除
    pub fn delete(&mut self, user_id: i64) -> Result<()> {
        if self.status == EntityStatus::Deleted {
            return Ok(()); // 已经被删除，不需要再次操作
        }

        self.status = EntityStatus::Deleted;
        self.changed_by = user_id;
        self.update_timestamp_and_version();
        Ok(())
    }

    // 获取ID
    pub fn id(&self) -> i64 {
        self.id
    }

    // 获取歌曲ID
    pub fn song_id(&self) -> i64 {
        self.song_id
    }

    // 更新时间戳和版本号
    fn update_timestamp_and_version(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        if self.version == self.old_version {
            self.version += 1;
        }
    }

    // 准备保存（版本控制）
    pub fn prepare_for_save(&mut self) {
        self.old_version = self.version;
    }

    // 是否被删除
    pub fn is_deleted(&self) -> bool {
        self.status == EntityStatus::Deleted
    }
}

// 仓储接口（依赖反转）
#[async_trait]
pub trait BookmarkRepository: Send + Sync {
    async fn save(&self, bookmark: &mut Bookmark) -> Result<Bookmark>;
    async fn find_by_id(&self, id: i64) -> Result<Bookmark>;
    async fn find_by_song_id(&self, song_id: i64) -> Result<Vec<Bookmark>>;
    async fn find_by_user_id(&self, user_id: i64) -> Result<Vec<Bookmark>>;
    async fn find_by_song_and_user(&self, song_id: i64, user_id: i64) -> Result<Vec<Bookmark>>;
    async fn delete_all(&self) -> Result<()>;
}

// 领域服务 - 只保留命令部分，去除查询方法
pub struct BookmarkService<R: BookmarkRepository> {
    repository: R,
    id_generator: Arc<dyn IdGenerator>,
}

impl<R: BookmarkRepository> BookmarkService<R> {
    pub fn new(repository: R, id_generator: Arc<dyn IdGenerator>) -> Self {
        Self {
            repository,
            id_generator,
        }
    }

    // 创建书签 - 命令
    pub async fn create_bookmark(
        &self,
        song_id: i64,
        position: i64,
        comment: String,
        user_id: i64,
    ) -> Result<Bookmark> {
        let mut bookmark = Bookmark::new(song_id, position, comment, user_id)?;

        // 使用ID生成器生成ID
        bookmark.id = self.id_generator.next_id().await?;

        self.repository.save(&mut bookmark).await
    }

    // 更新书签位置 - 命令
    pub async fn update_position(&self, id: i64, position: i64, user_id: i64) -> Result<Bookmark> {
        let mut bookmark = self.repository.find_by_id(id).await?;
        bookmark.update_position(position, user_id)?;
        self.repository.save(&mut bookmark).await
    }

    // 更新书签评论 - 命令
    pub async fn update_comment(&self, id: i64, comment: String, user_id: i64) -> Result<Bookmark> {
        let mut bookmark = self.repository.find_by_id(id).await?;
        bookmark.update_comment(comment, user_id)?;
        self.repository.save(&mut bookmark).await
    }

    // 删除书签 - 命令
    pub async fn delete_bookmark(&self, id: i64, user_id: i64) -> Result<Bookmark> {
        let mut bookmark = self.repository.find_by_id(id).await?;
        bookmark.delete(user_id)?;
        self.repository.save(&mut bookmark).await
    }

    // 批量删除书签 - 辅助方法
    async fn delete_bookmarks(&self, bookmarks: Vec<Bookmark>, user_id: i64) -> Result<()> {
        for mut bookmark in bookmarks {
            if !bookmark.is_deleted() {
                bookmark.delete(user_id)?;
                self.repository.save(&mut bookmark).await?;
            }
        }
        Ok(())
    }

    // 删除用户的所有书签 - 命令
    pub async fn delete_by_user(&self, user_id: i64) -> Result<()> {
        let bookmarks = self.repository.find_by_user_id(user_id).await?;
        self.delete_bookmarks(bookmarks, user_id).await
    }

    // 删除歌曲的所有书签 - 命令
    pub async fn delete_by_song(&self, song_id: i64, user_id: i64) -> Result<()> {
        let bookmarks = self.repository.find_by_song_id(song_id).await?;
        self.delete_bookmarks(bookmarks, user_id).await
    }

    // 清除所有书签 - 命令
    pub async fn clear(&self) -> Result<()> {
        self.repository.delete_all().await
    }
}
