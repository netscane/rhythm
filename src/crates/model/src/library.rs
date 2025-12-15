use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub remote_path: String,
    pub last_scan_at: i64,
    pub updated_at: i64,
    pub created_at: i64,
}
