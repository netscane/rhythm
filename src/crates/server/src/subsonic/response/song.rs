use super::directory::Child;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TopSongs {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub song: Vec<Child>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SimilarSongs {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub song: Vec<Child>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SimilarSongs2 {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub song: Vec<Child>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    #[serde(flatten)]
    pub child: Child,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Songs {
    pub song: Vec<Child>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SongsByGenre(pub Songs);

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RandomSongs(pub Songs);

/// 歌曲列表（扁平结构，用于扩展 API）
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SongList {
    pub song: Vec<Child>,
}
