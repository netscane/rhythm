use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MbzAlbum {
    pub mbz_album_id: Option<String>,
    pub mbz_album_artist_id: Option<String>,
    pub mbz_album_type: Option<String>,
    pub mbz_album_comment: Option<String>,
}
