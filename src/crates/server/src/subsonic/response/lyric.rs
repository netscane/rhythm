use serde::Serialize;
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StructuredLyric {
    pub lang: String,

    pub synced: bool,

    pub line: Vec<Line>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_artist: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub value: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    pub value: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LyricsList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_lyrics: Option<Vec<StructuredLyric>>,
}
