use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Line {
    pub start: Option<i64>,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Lyrics {
    pub display_artist: String,
    pub display_title: String,
    pub lang: String,
    pub line: Vec<Line>,
    pub offset: Option<i64>,
    pub synced: bool,
}
