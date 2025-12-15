use serde::Serialize;
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub username: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    pub scrobbling_enabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bit_rate: Option<i32>,

    pub admin_role: bool,

    pub settings_role: bool,

    pub download_role: bool,

    pub upload_role: bool,

    pub playlist_role: bool,

    pub cover_art_role: bool,

    pub comment_role: bool,

    pub podcast_role: bool,

    pub stream_role: bool,

    pub jukebox_role: bool,

    pub share_role: bool,

    pub video_conversion_role: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<Vec<i32>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Users {
    pub user: Vec<User>,
}
