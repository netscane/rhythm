use std::sync::Arc;

use crate::command::shared::IdGenerator;
use crate::error::AppError;
use domain::user::{User, UserRepository};
use domain::value::UserId;

pub trait PasswordHasher {
    fn hash(&self, plain: &str) -> Result<String, AppError>;
    fn verify(&self, pwd: &str, hashed_pwd: &str) -> Result<(), AppError>;
}

/// 密码加密器 trait（用于可逆加密，支持 Subsonic token 认证）
pub trait PasswordEncryptor: Send + Sync {
    fn encrypt(&self, plain_password: &str) -> Result<String, AppError>;
    fn decrypt(&self, encrypted_password: &str) -> Result<String, AppError>;
}

#[derive(Debug, Clone)]
pub struct UserClaims {
    pub user_name: String, // user name
    pub is_admin: bool,    // is admin
}

impl From<&User> for UserClaims {
    fn from(user: &User) -> Self {
        Self {
            user_name: user.username.clone(),
            is_admin: user.is_admin,
        }
    }
}

pub trait TokenService {
    fn issue(&self, claims: &UserClaims) -> Result<String, AppError>;
    fn verify(&self, token: &str) -> Result<UserClaims, AppError>;
}

#[derive(Clone)]
pub struct AuthService {
    user_repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    encryptor: Arc<dyn PasswordEncryptor>,
    token_svc: Arc<dyn TokenService>,
    id_generator: Arc<dyn IdGenerator>,
}

impl AuthService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        encryptor: Arc<dyn PasswordEncryptor>,
        token_svc: Arc<dyn TokenService>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            user_repo,
            hasher,
            encryptor,
            token_svc,
            id_generator,
        }
    }

    /// Login with username and password, returns JWT token
    pub async fn login(&self, username: &str, pwd: &str) -> Result<String, AppError> {
        let user = self
            .user_repo
            .find_by_username(username)
            .await?
            .ok_or_else(|| AppError::AuthError("invalid username".to_string()))?;
        self.hasher.verify(pwd, &user.password)?;
        self.token_svc.issue(&UserClaims::from(&user))
    }

    /// Authenticate with token, returns refreshed token
    pub async fn authenticate(&self, token: &str) -> Result<String, AppError> {
        let claims = self.token_svc.verify(token)?;
        let user = self
            .user_repo
            .find_by_username(&claims.user_name)
            .await?
            .ok_or_else(|| AppError::AuthError("invalid username".to_string()))?;
        self.token_svc.issue(&UserClaims::from(&user))
    }

    /// Create admin user if no users exist
    pub async fn create_admin(&self, username: &str, pwd: &str) -> Result<(), AppError> {
        if self.user_repo.count().await? > 0 {
            return Err(AppError::AuthError("can not create another admin".to_string()));
        }
        let hashed_pwd = self.hasher.hash(pwd)?;
        let encrypted_password = self.encryptor.encrypt(pwd)?;
        let id = UserId::from(self.id_generator.next_id().await?);
        let admin = User::new(id, username, None, "", true, &hashed_pwd, &encrypted_password)?;
        self.user_repo.save(&admin).await?;
        Ok(())
    }
}
