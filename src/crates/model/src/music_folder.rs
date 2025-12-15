use chrono::NaiveDateTime;
#[derive(Debug, Clone)]
pub struct MusicFolder {
    pub id: i64,
    pub name: String,
    pub last_scan_at: NaiveDateTime,
}
