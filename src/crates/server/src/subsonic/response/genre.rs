use serde::Serialize;
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ItemGenre {
    pub name: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub song_count: i32,

    pub album_count: i32,

    pub value: String,
}

impl From<model::genre::Genre> for Genre {
    fn from(genre: model::genre::Genre) -> Self {
        Self {
            value: genre.name,
            song_count: genre.song_count,
            album_count: genre.album_count,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Genres {
    pub genre: Vec<Genre>,
}

impl Genres {
    pub fn new(genres: Vec<model::genre::Genre>) -> Self {
        Self {
            genre: genres.into_iter().map(Genre::from).collect(),
        }
    }
}
