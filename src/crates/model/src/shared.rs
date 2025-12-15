use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ArtistSummary {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Annotation {
    pub play_count: i32,
    pub play_date: Option<NaiveDateTime>,
    pub rating: i32,
    pub starred: bool,
    pub starred_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenreSummary {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Contributor {
    pub artist_id: i64,
    pub role: String,
    pub sub_role: Option<String>,
    pub artist_name: String,
}
