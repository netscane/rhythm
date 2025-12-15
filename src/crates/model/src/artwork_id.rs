use crate::kind::Kind;

#[derive(Debug)]
pub struct ArtworkID {
    pub kind: Kind,
    pub id: String,
    pub last_update: i64,
}
