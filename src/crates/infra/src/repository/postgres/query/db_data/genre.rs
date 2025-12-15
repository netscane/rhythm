use model::genre::Genre;
use sea_orm::FromQueryResult;
#[derive(FromQueryResult, Debug)]
pub struct GenreModel {
    pub name: String,
    pub song_count: i32,
    pub album_count: i32,
}

impl From<GenreModel> for Genre {
    fn from(model: GenreModel) -> Self {
        Self {
            name: model.name,
            song_count: model.song_count,
            album_count: model.album_count,
        }
    }
}
