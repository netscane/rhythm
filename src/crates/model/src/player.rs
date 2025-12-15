use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub user_agent: String,
    pub user_name: String,
    pub client: String,
    pub ip_address: String,
    pub last_seen: i64,
    pub transcoding_id: String,
    pub max_bit_rate: i32,
    pub report_real_path: bool,
    pub scrobble_enabled: bool,
}
