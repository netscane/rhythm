use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Transcoding {
    pub id: String,
    pub name: String,
    pub target_format: String,
    pub command: String,
    pub default_bit_rate: i32,
}
