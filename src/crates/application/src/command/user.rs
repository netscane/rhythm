use crate::error::AppError;
use domain::user::{User, UserRepository};
use domain::value::UserId;
use std::sync::Arc;

/// 创建用户命令
pub struct CreateUserCmd {
    pub username: String,
    pub password: String,            // 已经哈希过的密码 (bcrypt)
    pub encrypted_password: String,  // AES-256-GCM 加密的原始密码，用于 Subsonic token 认证
    pub email: String,
    pub is_admin: bool,
}

/// 更新用户命令
pub struct UpdateUserCmd {
    pub username: String,
    pub password: Option<String>,            // 已经哈希过的密码，None 表示不修改
    pub encrypted_password: Option<String>,  // 加密的原始密码，None 表示不修改
    pub email: Option<String>,
    pub is_admin: Option<bool>,
}

/// 删除用户命令
pub struct DeleteUserCmd {
    pub username: String,
}

/// 修改密码命令
pub struct ChangePasswordCmd {
    pub username: String,
    pub password: String,            // 已经哈希过的密码 (bcrypt)
    pub encrypted_password: String,  // AES-256-GCM 加密的原始密码，用于 Subsonic token 认证
}

/// 用户应用服务
pub struct UserAppService {
    user_repo: Arc<dyn UserRepository>,
    id_generator: Arc<dyn crate::command::shared::IdGenerator>,
}

impl UserAppService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        id_generator: Arc<dyn crate::command::shared::IdGenerator>,
    ) -> Self {
        Self {
            user_repo,
            id_generator,
        }
    }

    /// 创建新用户
    pub async fn create_user(&self, cmd: CreateUserCmd) -> Result<(), AppError> {
        // 检查用户名是否已存在
        let existing = self.user_repo.find_by_username(&cmd.username).await;
        if let Ok(Some(_)) = existing {
            return Err(AppError::InvalidInput(format!(
                "Username '{}' already exists",
                cmd.username
            )));
        }

        // 生成新的用户 ID
        let user_id = UserId::from(self.id_generator.next_id().await?);

        // 创建用户领域对象
        let user = User::new(
            user_id,
            &cmd.username,
            None, // name 默认与 username 相同
            &cmd.email,
            cmd.is_admin,
            &cmd.password,
            &cmd.encrypted_password,
        )
        .map_err(|e| AppError::UnknownError(e.to_string()))?;

        // 保存用户
        self.user_repo.save(&user).await?;

        Ok(())
    }

    /// 更新用户
    pub async fn update_user(&self, cmd: UpdateUserCmd) -> Result<(), AppError> {
        // 查找用户
        let user = self
            .user_repo
            .find_by_username(&cmd.username)
            .await?
            .ok_or_else(|| {
                AppError::AggregateNotFound("User".to_string(), cmd.username.clone())
            })?;

        let mut user = user;

        // 更新密码
        if let (Some(password), Some(encrypted_password)) = (&cmd.password, &cmd.encrypted_password) {
            user.change_password(password, encrypted_password)
                .map_err(|e| AppError::UnknownError(e.to_string()))?;
        }

        // 更新邮箱
        if let Some(email) = &cmd.email {
            user.update_profile(None, Some(email));
        }

        // 更新管理员角色
        if let Some(is_admin) = cmd.is_admin {
            user.is_admin = is_admin;
        }

        // 保存用户
        self.user_repo.save(&user).await?;

        Ok(())
    }

    /// 删除用户
    pub async fn delete_user(&self, cmd: DeleteUserCmd) -> Result<(), AppError> {
        // 删除用户
        self.user_repo.delete(&cmd.username).await?;

        Ok(())
    }

    /// 修改密码
    pub async fn change_password(&self, cmd: ChangePasswordCmd) -> Result<(), AppError> {
        // 查找用户
        let user = self
            .user_repo
            .find_by_username(&cmd.username)
            .await?
            .ok_or_else(|| {
                AppError::AggregateNotFound("User".to_string(), cmd.username.clone())
            })?;

        let mut user = user;

        // 更新密码
        user.change_password(&cmd.password, &cmd.encrypted_password)
            .map_err(|e| AppError::UnknownError(e.to_string()))?;

        // 保存用户
        self.user_repo.save(&user).await?;

        Ok(())
    }
}
