use super::artist::ArtistID3Ref;
use super::genre::ItemGenre;
use application::query::dto::cover_art::{album_cover_art_id, audio_file_cover_art_id};
use chrono::NaiveDateTime;
use serde::Serialize;

impl From<model::audio_file::AudioFile> for Child {
    fn from(audio_file: model::audio_file::AudioFile) -> Self {
        let cover_art = audio_file_cover_art_id(audio_file.id);

        Self {
            id: audio_file.id.to_string(),
            parent: Some(audio_file.album_id.to_string()),
            is_dir: false,
            title: audio_file.title.clone(),
            name: audio_file.name.clone(),
            album: Some(audio_file.album.clone()),
            artist: Some(audio_file.artist.name.clone()),
            track: if audio_file.track_number > 0 {
                Some(audio_file.track_number)
            } else {
                None
            },
            year: audio_file.year,
            genre: audio_file.genre.as_ref().map(|g| g.name.clone()),
            cover_art: Some(cover_art),
            size: Some(audio_file.size),
            content_type: Some(format!("audio/{}", audio_file.suffix)),
            suffix: Some(audio_file.suffix.clone()),
            starred: audio_file.annotation.starred_at,
            transcoded_content_type: None,
            transcoded_suffix: None,
            duration: audio_file.duration,
            bit_rate: Some(audio_file.bit_rate),
            path: Some(audio_file.path.clone()),
            play_count: audio_file.annotation.play_count,
            disk_number: audio_file.disc_number,
            created: Some(audio_file.created_at),
            album_id: Some(audio_file.album_id.to_string()),
            artist_id: Some(audio_file.artist.id.to_string()),
            r#type: Some("music".to_string()),
            user_rating: Some(audio_file.annotation.rating),
            song_count: audio_file.song_count,
            is_video: false,
        }
    }
}

impl From<model::album::Album> for Child {
    fn from(album: model::album::Album) -> Self {
        Self {
            id: format!("al-{}", album.id),
            parent: None,
            is_dir: true,
            title: album.name.clone(),
            name: album.name.clone(),
            album: Some(album.name.clone()),
            artist: Some(album.artist.name.clone()),
            track: None,
            year: album.year,
            genre: album.genre.as_ref().map(|g| g.name.clone()),
            cover_art: Some(album_cover_art_id(album.id)),
            size: Some(album.size),
            content_type: None,
            suffix: None,
            starred: album.annotation.starred_at,
            transcoded_content_type: None,
            transcoded_suffix: None,
            duration: album.duration,
            bit_rate: None,
            path: None,
            play_count: album.annotation.play_count,
            disk_number: 0,
            created: Some(album.created_at),
            album_id: Some(album.id.to_string()),
            artist_id: Some(album.artist.id.to_string()),
            r#type: Some("album".to_string()),
            user_rating: Some(album.annotation.rating),
            song_count: album.song_count,
            is_video: false,
        }
    }
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_rating: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_rating: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_count: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub child: Option<Vec<Child>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpenSubsonicChild {
    pub played: Option<NaiveDateTime>,
    pub bpm: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub sort_name: String,
    pub media_type: String,
    pub genres: Vec<ItemGenre>,

    pub channel_count: i32,
    pub sampling_rate: i32,

    pub artists: Vec<ArtistID3Ref>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Child {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,

    pub is_dir: bool,

    pub title: String,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<NaiveDateTime>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcoded_content_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcoded_suffix: Option<String>,

    pub duration: i64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_rate: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    pub play_count: i32,

    pub disk_number: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<NaiveDateTime>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_rating: Option<i32>,

    pub song_count: i32,

    pub is_video: bool,
}
