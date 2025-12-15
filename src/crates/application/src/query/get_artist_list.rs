use crate::query::dao::ArtistDao;
use crate::query::QueryError;
use model::artist::Artist;
use std::sync::Arc;

pub struct GetArtistList {
    artist_dao: Arc<dyn ArtistDao + Send + Sync>,
}

impl GetArtistList {
    pub fn new(artist_dao: Arc<dyn ArtistDao + Send + Sync>) -> Self {
        Self { artist_dao }
    }

    pub async fn handle(&self, list_type: &str, limit: i32) -> Result<Vec<Artist>, QueryError> {
        match list_type {
            "frequent" | "mostPlayed" => self.artist_dao.get_most_played(limit).await,
            "recent" | "recentlyPlayed" => self.artist_dao.get_recently_played(limit).await,
            _ => Err(QueryError::InvalidInput(format!(
                "Unknown list type: {}",
                list_type
            ))),
        }
    }
}
