use chrono::NaiveDateTime;
use model::music_folder::MusicFolder;
use sea_orm::FromQueryResult;
#[derive(FromQueryResult, Debug)]
pub struct MusicFolderModel {
    pub id: i64,
    pub name: String,
    pub last_scan_at: NaiveDateTime,
}

impl From<MusicFolderModel> for MusicFolder {
    fn from(model: MusicFolderModel) -> Self {
        Self {
            id: model.id,
            name: model.name,
            last_scan_at: model.last_scan_at,
        }
    }
}
