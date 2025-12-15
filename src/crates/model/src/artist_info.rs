use super::artist::Artist;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtistInfo {
    pub id: String,
    pub name: String,
    pub mbid: String,
    pub biography: String,
    pub small_image_url: String,
    pub medium_image_url: String,
    pub large_image_url: String,
    pub last_fm_url: String,
    pub similar_artists: Vec<Artist>,
}
