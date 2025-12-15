use super::album::AlbumID3;
use super::artist::Artist;
use super::artist::ArtistID3;
use super::directory::Child;
use serde::Serialize;

/// SearchResult for deprecated search API (Since 1.0.0, deprecated 1.4.0)
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// Total number of matches
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_hits: Option<i64>,

    /// Offset of the result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,

    /// Matched items (files/directories)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<Vec<Child>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult2 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<Vec<Artist>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<Vec<Child>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub song: Option<Vec<Child>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult3 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<Vec<ArtistID3>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<Vec<AlbumID3>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub song: Option<Vec<Child>>,
}
