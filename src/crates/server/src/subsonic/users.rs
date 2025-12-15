use crate::subsonic::response::error::SubsonicError;
use crate::subsonic::response::Subsonic;
use crate::AppState;
use actix_web::{web, HttpMessage, HttpRequest};
use application::auth::PasswordEncryptor;
use application::command::user::{ChangePasswordCmd, CreateUserCmd, DeleteUserCmd, UpdateUserCmd, UserAppService};
use infra::auth::BcryptPasswordHasher;
use infra::repository::postgres::command::user::UserRepositoryImpl;
use infra::Aes256GcmEncryptor;
use serde::Deserialize;
use std::sync::Arc;

/// createUser API 请求参数
///
/// 根据 Subsonic 规范 (Since 1.1.0):
/// - username: 新用户的用户名（必需）
/// - password: 新用户的密码，明文或 hex 编码（必需）
/// - email: 新用户的邮箱地址（必需）
/// - ldapAuthenticated: 是否使用 LDAP 认证（可选，默认 false）
/// - adminRole: 是否为管理员（可选，默认 false）
/// - settingsRole: 是否允许修改个人设置和密码（可选，默认 true）
/// - streamRole: 是否允许播放文件（可选，默认 true）
/// - jukeboxRole: 是否允许使用 jukebox 模式（可选，默认 false）
/// - downloadRole: 是否允许下载文件（可选，默认 false）
/// - uploadRole: 是否允许上传文件（可选，默认 false）
/// - playlistRole: 是否允许创建和删除播放列表（可选，默认 false，1.8.0 后无效）
/// - coverArtRole: 是否允许修改封面和标签（可选，默认 false）
/// - commentRole: 是否允许创建和编辑评论和评分（可选，默认 false）
/// - podcastRole: 是否允许管理 Podcast（可选，默认 false）
/// - shareRole: 是否允许分享文件（可选，默认 false，Since 1.8.0）
/// - videoConversionRole: 是否允许启动视频转换（可选，默认 false，Since 1.15.0）
/// - musicFolderId: 允许访问的音乐文件夹 ID（可选，默认所有文件夹，Since 1.12.0）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserQuery {
    /// 新用户的用户名
    pub username: String,

    /// 新用户的密码（明文或 hex 编码）
    pub password: String,

    /// 新用户的邮箱地址
    pub email: String,

    /// 是否使用 LDAP 认证
    #[serde(default)]
    pub ldap_authenticated: bool,

    /// 是否为管理员
    #[serde(default)]
    pub admin_role: bool,

    /// 是否允许修改个人设置和密码
    #[serde(default = "default_true")]
    pub settings_role: bool,

    /// 是否允许播放文件
    #[serde(default = "default_true")]
    pub stream_role: bool,

    /// 是否允许使用 jukebox 模式
    #[serde(default)]
    pub jukebox_role: bool,

    /// 是否允许下载文件
    #[serde(default)]
    pub download_role: bool,

    /// 是否允许上传文件
    #[serde(default)]
    pub upload_role: bool,

    /// 是否允许创建和删除播放列表（1.8.0 后无效）
    #[serde(default)]
    pub playlist_role: bool,

    /// 是否允许修改封面和标签
    #[serde(default)]
    pub cover_art_role: bool,

    /// 是否允许创建和编辑评论和评分
    #[serde(default)]
    pub comment_role: bool,

    /// 是否允许管理 Podcast
    #[serde(default)]
    pub podcast_role: bool,

    /// 是否允许分享文件
    #[serde(default)]
    pub share_role: bool,

    /// 是否允许启动视频转换
    #[serde(default)]
    pub video_conversion_role: bool,

    /// 允许访问的音乐文件夹 ID
    #[serde(default)]
    pub music_folder_id: Vec<i32>,
}

fn default_true() -> bool {
    true
}

/// 解码密码
///
/// 根据 Subsonic 规范，密码可以是明文或以 "enc:" 前缀的 hex 编码
fn decode_password(password: &str) -> Result<String, SubsonicError> {
    if let Some(hex_str) = password.strip_prefix("enc:") {
        // Hex 编码的密码
        let bytes = hex::decode(hex_str)
            .map_err(|e| SubsonicError::error_generic().wrap(format!("Invalid hex password: {}", e)))?;
        String::from_utf8(bytes)
            .map_err(|e| SubsonicError::error_generic().wrap(format!("Invalid UTF-8 in password: {}", e)))
    } else {
        // 明文密码
        Ok(password.to_string())
    }
}

/// createUser - 创建新用户
///
/// 根据 Subsonic 规范 (Since 1.1.0):
/// - 在服务器上创建新用户
/// - 需要管理员权限
pub async fn create_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<CreateUserQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取当前用户
    let current_user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 检查当前用户是否为管理员
    if !current_user.is_admin {
        return Err(SubsonicError::error_authorization_fail()
            .wrap("Only administrators can create users".to_string()));
    }

    // 解码密码
    let plain_password = decode_password(&query.password)?;

    // 哈希密码 (bcrypt for normal auth)
    let hasher = BcryptPasswordHasher::new(12);
    let hashed_password = hasher
        .hash(&plain_password)
        .map_err(|e| SubsonicError::error_generic().wrap(format!("Failed to hash password: {}", e)))?;
    
    // 加密密码 (AES-256-GCM for subsonic token auth)
    let encryptor = Aes256GcmEncryptor::new(&state.app_cfg.password_encryption_key())
        .map_err(|e| SubsonicError::error_generic().wrap(format!("Failed to create encryptor: {}", e)))?;
    let encrypted_password = encryptor
        .encrypt(&plain_password)
        .map_err(|e| SubsonicError::error_generic().wrap(format!("Failed to encrypt password: {}", e)))?;

    // 创建用户服务
    let user_repo: Arc<dyn domain::user::UserRepository> =
        Arc::new(UserRepositoryImpl::new(state.db.clone()));
    let user_service = UserAppService::new(user_repo, state.id_generator.clone());

    // 创建用户
    user_service
        .create_user(CreateUserCmd {
            username: query.username.clone(),
            password: hashed_password,
            encrypted_password,
            email: query.email.clone(),
            is_admin: query.admin_role,
        })
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    Ok(Subsonic::default())
}

/// updateUser API 请求参数
///
/// 根据 Subsonic 规范 (Since 1.10.1):
/// - username: 用户名（必需）
/// - password: 密码，明文或 hex 编码（可选）
/// - email: 邮箱地址（可选）
/// - ldapAuthenticated: 是否使用 LDAP 认证（可选）
/// - adminRole: 是否为管理员（可选）
/// - settingsRole: 是否允许修改个人设置和密码（可选）
/// - streamRole: 是否允许播放文件（可选）
/// - jukeboxRole: 是否允许使用 jukebox 模式（可选）
/// - downloadRole: 是否允许下载文件（可选）
/// - uploadRole: 是否允许上传文件（可选）
/// - coverArtRole: 是否允许修改封面和标签（可选）
/// - commentRole: 是否允许创建和编辑评论和评分（可选）
/// - podcastRole: 是否允许管理 Podcast（可选）
/// - shareRole: 是否允许分享文件（可选）
/// - videoConversionRole: 是否允许启动视频转换（可选，Since 1.15.0）
/// - musicFolderId: 允许访问的音乐文件夹 ID（可选，Since 1.12.0）
/// - maxBitRate: 最大比特率（可选，Since 1.13.0）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserQuery {
    /// 用户名
    pub username: String,

    /// 密码（明文或 hex 编码）
    #[serde(default)]
    pub password: Option<String>,

    /// 邮箱地址
    #[serde(default)]
    pub email: Option<String>,

    /// 是否使用 LDAP 认证
    #[serde(default)]
    pub ldap_authenticated: Option<bool>,

    /// 是否为管理员
    #[serde(default)]
    pub admin_role: Option<bool>,

    /// 是否允许修改个人设置和密码
    #[serde(default)]
    pub settings_role: Option<bool>,

    /// 是否允许播放文件
    #[serde(default)]
    pub stream_role: Option<bool>,

    /// 是否允许使用 jukebox 模式
    #[serde(default)]
    pub jukebox_role: Option<bool>,

    /// 是否允许下载文件
    #[serde(default)]
    pub download_role: Option<bool>,

    /// 是否允许上传文件
    #[serde(default)]
    pub upload_role: Option<bool>,

    /// 是否允许修改封面和标签
    #[serde(default)]
    pub cover_art_role: Option<bool>,

    /// 是否允许创建和编辑评论和评分
    #[serde(default)]
    pub comment_role: Option<bool>,

    /// 是否允许管理 Podcast
    #[serde(default)]
    pub podcast_role: Option<bool>,

    /// 是否允许分享文件
    #[serde(default)]
    pub share_role: Option<bool>,

    /// 是否允许启动视频转换
    #[serde(default)]
    pub video_conversion_role: Option<bool>,

    /// 允许访问的音乐文件夹 ID
    #[serde(default)]
    pub music_folder_id: Vec<i32>,

    /// 最大比特率 (Kbps)
    #[serde(default)]
    pub max_bit_rate: Option<i32>,
}

/// updateUser - 更新用户
///
/// 根据 Subsonic 规范 (Since 1.10.1):
/// - 修改服务器上已存在的用户
/// - 需要管理员权限
pub async fn update_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<UpdateUserQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取当前用户
    let current_user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 检查当前用户是否为管理员
    if !current_user.is_admin {
        return Err(SubsonicError::error_authorization_fail()
            .wrap("Only administrators can update users".to_string()));
    }

    // 解码并处理密码（如果提供）
    let (hashed_password, encrypted_password) = if let Some(ref password) = query.password {
        let plain_password = decode_password(password)?;
        let hasher = BcryptPasswordHasher::new(12);
        let hashed = hasher
            .hash(&plain_password)
            .map_err(|e| SubsonicError::error_generic().wrap(format!("Failed to hash password: {}", e)))?;
        
        let encryptor = Aes256GcmEncryptor::new(&state.app_cfg.password_encryption_key())
            .map_err(|e| SubsonicError::error_generic().wrap(format!("Failed to create encryptor: {}", e)))?;
        let encrypted = encryptor
            .encrypt(&plain_password)
            .map_err(|e| SubsonicError::error_generic().wrap(format!("Failed to encrypt password: {}", e)))?;
        
        (Some(hashed), Some(encrypted))
    } else {
        (None, None)
    };

    // 创建用户服务
    let user_repo: Arc<dyn domain::user::UserRepository> =
        Arc::new(UserRepositoryImpl::new(state.db.clone()));
    let user_service = UserAppService::new(user_repo, state.id_generator.clone());

    // 更新用户
    user_service
        .update_user(UpdateUserCmd {
            username: query.username.clone(),
            password: hashed_password,
            encrypted_password,
            email: query.email.clone(),
            is_admin: query.admin_role,
        })
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    Ok(Subsonic::default())
}

/// deleteUser API 请求参数
///
/// 根据 Subsonic 规范 (Since 1.3.0):
/// - username: 要删除的用户名（必需）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUserQuery {
    /// 要删除的用户名
    pub username: String,
}

/// deleteUser - 删除用户
///
/// 根据 Subsonic 规范 (Since 1.3.0):
/// - 删除服务器上已存在的用户
/// - 需要管理员权限
pub async fn delete_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<DeleteUserQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取当前用户
    let current_user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 检查当前用户是否为管理员
    if !current_user.is_admin {
        return Err(SubsonicError::error_authorization_fail()
            .wrap("Only administrators can delete users".to_string()));
    }

    // 不允许删除自己
    if current_user.username == query.username {
        return Err(SubsonicError::error_generic()
            .wrap("Cannot delete yourself".to_string()));
    }

    // 创建用户服务
    let user_repo: Arc<dyn domain::user::UserRepository> =
        Arc::new(UserRepositoryImpl::new(state.db.clone()));
    let user_service = UserAppService::new(user_repo, state.id_generator.clone());

    // 删除用户
    user_service
        .delete_user(DeleteUserCmd {
            username: query.username.clone(),
        })
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    Ok(Subsonic::default())
}

/// changePassword API 请求参数
///
/// 根据 Subsonic 规范 (Since 1.1.0):
/// - username: 要修改密码的用户名（必需）
/// - password: 新密码，明文或 hex 编码（必需）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordQuery {
    /// 要修改密码的用户名
    pub username: String,

    /// 新密码（明文或 hex 编码）
    pub password: String,
}

/// changePassword - 修改密码
///
/// 根据 Subsonic 规范 (Since 1.1.0):
/// - 修改用户密码
/// - 普通用户只能修改自己的密码，管理员可以修改任何用户的密码
pub async fn change_password(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ChangePasswordQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取当前用户
    let current_user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 检查权限：只能修改自己的密码，除非是管理员
    if current_user.username != query.username && !current_user.is_admin {
        return Err(SubsonicError::error_authorization_fail()
            .wrap("You can only change your own password".to_string()));
    }

    // 解码密码
    let plain_password = decode_password(&query.password)?;

    // 哈希密码 (bcrypt for normal auth)
    let hasher = BcryptPasswordHasher::new(12);
    let hashed_password = hasher
        .hash(&plain_password)
        .map_err(|e| SubsonicError::error_generic().wrap(format!("Failed to hash password: {}", e)))?;
    
    // 加密密码 (AES-256-GCM for subsonic token auth)
    let encryptor = Aes256GcmEncryptor::new(&state.app_cfg.password_encryption_key())
        .map_err(|e| SubsonicError::error_generic().wrap(format!("Failed to create encryptor: {}", e)))?;
    let encrypted_password = encryptor
        .encrypt(&plain_password)
        .map_err(|e| SubsonicError::error_generic().wrap(format!("Failed to encrypt password: {}", e)))?;

    // 创建用户服务
    let user_repo: Arc<dyn domain::user::UserRepository> =
        Arc::new(UserRepositoryImpl::new(state.db.clone()));
    let user_service = UserAppService::new(user_repo, state.id_generator.clone());

    // 修改密码
    user_service
        .change_password(ChangePasswordCmd {
            username: query.username.clone(),
            password: hashed_password,
            encrypted_password,
        })
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    Ok(Subsonic::default())
}
