use serde::Serialize;
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MusicFolder {
    pub id: i64,

    pub name: String,
}

impl From<model::music_folder::MusicFolder> for MusicFolder {
    fn from(music_folder: model::music_folder::MusicFolder) -> Self {
        Self {
            id: music_folder.id,
            name: music_folder.name,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MusicFolders {
    pub music_folder: Vec<MusicFolder>,
}

impl MusicFolders {
    pub fn new(music_folders: Vec<model::music_folder::MusicFolder>) -> Self {
        Self {
            music_folder: music_folders.into_iter().map(MusicFolder::from).collect(),
        }
    }
}
