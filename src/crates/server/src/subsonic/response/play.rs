use super::directory::Child;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub entry: Option<Vec<NowPlayingEntry>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NowPlayingEntry {
    #[serde(flatten)]
    pub child: Child,

    pub username: String,

    pub minutes_ago: i32,

    pub player_id: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_name: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpenSubsonicExtension {
    pub name: String,

    pub versions: Vec<i32>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,

    pub song_count: i32,

    pub duration: i32,

    pub created: String,

    pub changed: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_user: Option<Vec<String>>, // Added to match the sequence element
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Playlists {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist: Option<Vec<Playlist>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistWithSongs {
    #[serde(flatten)]
    pub playlist: Playlist,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<Vec<Child>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayQueue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<Vec<Child>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,

    pub username: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed: Option<String>,

    pub changed_by: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RandomSongs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song: Option<Vec<Child>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReplayGain {
    pub track_gain: f64,

    pub track_peak: f64,

    pub album_gain: f64,

    pub album_peak: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_gain: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_gain: Option<f64>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JukeboxPlaylist {
    #[serde(flatten)]
    pub status: JukeboxStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<Vec<Child>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JukeboxStatus {
    pub current_index: i32,

    pub playing: bool,

    pub gain: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
}
