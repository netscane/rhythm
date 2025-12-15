use super::album::AlbumID3;
use super::artist::{Artist, ArtistID3};
use super::directory::Child;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Starred {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<Vec<Artist>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<Vec<Child>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub song: Option<Vec<Child>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Starred2 {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artist: Vec<ArtistID3>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub album: Vec<AlbumID3>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub song: Vec<Child>,
}
