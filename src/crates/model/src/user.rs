use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub user_name: String,
    pub name: String,
    pub email: String,
    pub is_admin: bool,
    pub last_login_at: Option<i64>,
    pub last_access_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub password: String,
    pub new_password: String,
    pub current_password: String,
}
