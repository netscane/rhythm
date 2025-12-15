use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    MediaFile,
    Artist,
    Album,
    Playlist,
}

impl Kind {
    fn prefix(&self) -> &'static str {
        match self {
            Kind::MediaFile => "mf",
            Kind::Artist => "ar",
            Kind::Album => "al",
            Kind::Playlist => "pl",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Kind::MediaFile => "media_file",
            Kind::Artist => "artist",
            Kind::Album => "album",
            Kind::Playlist => "playlist",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
