use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Radio {
    pub id: String,
    pub stream_url: String,
    pub name: String,
    pub home_page_url: String,
    pub created_at: i64,
    pub updated_at: i64,
}
