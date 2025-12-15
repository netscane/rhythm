use super::value::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use thiserror::Error;

// 用户事件定义
/// 用户领域事件
///
/// 用户领域中发生的事件，可被其他上下文订阅和处理。
#[derive(Debug, Clone)]
pub enum UserEvent {
    /// 用户被删除事件
    UserDeleted { id: i64, username: String },
    // 可以根据需要添加其他事件类型
}

/// 用户领域错误
///
/// 用户领域中可能发生的所有错误类型。
#[derive(Error, Debug)]
pub enum UserError {
    #[error("invalid user or password!{0}")]
    InvalidUserOrPassword(String),
    #[error("user not found: {0}")]
    UserNotFound(String),
    #[error("user is deleted")]
    UserDeleted,
    #[error("version conflict: {0}")]
    VersionConflictErr(i64),
    #[error("{0}")]
    DbErr(String),
    #[error("{0}")]
    OtherErr(String),
    #[error("{0}")]
    AuthError(String),
}

#[derive(Debug, Clone, PartialEq)]
#[repr(i32)]
pub enum UserStatus {
    Active = 1,
    New = 2,
    Deleted = 3,
}

impl From<UserStatus> for i32 {
    fn from(value: UserStatus) -> Self {
        value as i32
    }
}

impl TryFrom<i32> for UserStatus {
    type Error = String;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(UserStatus::Active),
            2 => Ok(UserStatus::New),
            3 => Ok(UserStatus::Deleted),
            _ => Err(format!("invalid value:{}", value)),
        }
    }
}

/// 用户聚合根
///
/// 用户是系统中的核心聚合根，代表有权访问系统的个体。
/// 用户可以登录系统、访问内容，并根据权限执行各种操作。
/// 用户聚合根管理其自身数据的完整性和生命周期。
#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,                       // 用户唯一标识符
    pub username: String,                 // 用户登录名，唯一
    pub name: String,                     // 用户昵称/显示名称
    pub email: String,                    // 用户电子邮件地址
    pub is_admin: bool,                   // 用户是否为管理员
    pub password: String,                 // 加密后的密码 (bcrypt)
    pub encrypted_password: String,       // AES-256-GCM 加密的原始密码，用于 Subsonic token 认证
    pub last_login_at: NaiveDateTime,     // 最后登录时间
    pub last_access_at: NaiveDateTime,    // 最后访问时间
    pub last_op_time: NaiveDateTime,      // 新增: 表示command的时间
    pub status: UserStatus,               // 用户状态
    pub version: i64,                     // 当前版本，用于乐观锁
    pub pending_events: Vec<UserEvent>,   // 用户事件列表
}

impl User {
    pub fn new(
        id: UserId,
        username: &str,
        name: Option<&str>,
        email: &str,
        is_admin: bool,
        hashed_password: &str,
        encrypted_password: &str,
    ) -> Result<Self, UserError> {
        Ok(User {
            id: UserId::from(id),
            username: String::from(username),
            name: {
                if let Some(name) = name {
                    String::from(name)
                } else {
                    String::from(username)
                }
            },
            email: String::from(email),
            is_admin,
            password: String::from(hashed_password),
            encrypted_password: String::from(encrypted_password),
            last_login_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap().naive_utc(),
            last_access_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap().naive_utc(),
            last_op_time: Local::now().naive_utc(),
            status: UserStatus::New,
            version: 0,
            pending_events: Vec::new(),
        })
    }

    pub fn change_password(&mut self, new_hashed_password: &str, new_encrypted_password: &str) -> Result<&mut Self, UserError> {
        self.password = String::from(new_hashed_password);
        self.encrypted_password = String::from(new_encrypted_password);
        Ok(self)
    }

    pub fn update_profile(&mut self, name: Option<&str>, email: Option<&str>) -> &mut Self {
        if let Some(name) = name {
            self.name = String::from(name);
        }

        if let Some(email) = email {
            self.email = String::from(email);
        }

        self
    }

    pub fn is_active(&self) -> Result<(), UserError> {
        if self.status == UserStatus::Deleted {
            return Err(UserError::UserDeleted);
        }
        Ok(())
    }

    // 从事件队列中拉取所有事件
    pub fn take_events(&mut self) -> Vec<UserEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

/// 用户仓储接口
///
/// 依赖反转原则 (DIP) 的体现。定义领域需要的仓储能力，
/// 由基础设施层实现。使领域逻辑不直接依赖于具体的数据访问技术。
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// 获取用户总数
    async fn count(&self) -> Result<u64, UserError>;

    /// 根据用户名查找用户
    async fn find_by_username<'a>(&'a self, username: &'a str) -> Result<Option<User>, UserError>;

    /// 根据用户ID查找用户
    async fn find_by_id<'a>(&'a self, id: UserId) -> Result<Option<User>, UserError>;

    /// 保存用户（创建或更新）
    async fn save<'a>(&'a self, user: &User) -> Result<(), UserError>;

    /// 删除用户
    async fn delete<'a>(&'a self, username: &'a str) -> Result<(), UserError>;
}

/*
/// 令牌值对象 (VO)
///
/// 传输到客户端的值对象，包含用户信息和认证数据。
/// 与实体不同，值对象没有标识符和生命周期，仅用于数据传输。
pub struct TokenVO {
    pub id: i64,                         // 用户ID
    pub name: String,                    // 用户名称
    pub username: String,                // 用户登录名
    pub is_admin: bool,                  // 是否管理员
    pub avatar: Option<String>,          // 头像URL，可选
    pub last_fm_api_key: Option<String>, // LastFM API密钥，可选
    pub subsonic_salt: String,           // Subsonic认证盐值
    pub subsonic_token: String,          // Subsonic认证令牌
}

*/
