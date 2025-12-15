use crate::subsonic::response::artist::ArtistID3Ref;
use crate::subsonic::response::directory::Child;
use crate::subsonic::response::genre::ItemGenre;
use application::query::dto::cover_art::album_cover_art_id;
use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]

pub struct DiscTitle {
    title: String,
    disc_number: i32,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpenSubsonicAlbumID3 {
    played: Option<NaiveDateTime>,
    user_rating: i32,
    genres: Vec<ItemGenre>,
    is_compilation: bool,
    pub sort_name: String,
    disk_titles: Vec<DiscTitle>,
    artists: Vec<ArtistID3Ref>,
}

impl OpenSubsonicAlbumID3 {
    pub fn new(album: model::album::Album) -> Self {
        Self {
            played: album.annotation.play_date,
            user_rating: album.annotation.rating,
            genres: album
                .genres
                .into_iter()
                .map(|genre| ItemGenre { name: genre.name })
                .collect(),
            is_compilation: album.compilation,
            sort_name: album.sort_name,
            disk_titles: album
                .discs
                .iter()
                .map(|(disc_number, title)| DiscTitle {
                    title: title.clone(),
                    disc_number: *disc_number,
                })
                .collect(),
            artists: album
                .contributors
                .into_iter()
                .map(|contributor| ArtistID3Ref {
                    id: contributor.artist_id.to_string(),
                    name: contributor.artist_name,
                })
                .collect(),
        }
    }
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AlbumID3 {
    pub id: String,

    pub name: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub artist: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub artist_id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub cover_art: String,

    pub song_count: i32,

    pub duration: i64,

    pub play_count: i32,

    pub created: NaiveDateTime,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<NaiveDateTime>,

    pub year: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,

    #[serde(flatten)]
    pub os_album_id3: OpenSubsonicAlbumID3,
}

impl AlbumID3 {
    pub fn new(album: model::album::Album) -> Self {
        Self {
            id: album.id.to_string(),
            name: album.name.clone(),
            artist: album.artist.name.clone(),
            artist_id: album.artist.id.to_string(),
            cover_art: album_cover_art_id(album.id),
            song_count: album.song_count,
            duration: album.duration,
            play_count: album.annotation.play_count,
            created: album.created_at,
            starred: album.annotation.starred_at,
            year: album.year.unwrap_or(0),
            genre: album
                .genre
                .as_ref()
                .map(|genre| genre.name.clone())
                .or(None),
            os_album_id3: OpenSubsonicAlbumID3::new(album),
        }
    }
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AlbumWithSongsID3 {
    #[serde(flatten)]
    pub album: AlbumID3,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub song: Vec<Child>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AlbumInfo {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub notes: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub music_brainz_id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_fm_url: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub small_image_url: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub medium_image_url: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub large_image_url: String,
}

impl AlbumInfo {
    pub fn new(
        album_info: model::album::AlbumInfo,
        small_image_url: String,
        medium_image_url: String,
        large_image_url: String,
    ) -> Self {
        Self {
            notes: album_info.description.unwrap_or_default(),
            music_brainz_id: "".to_string(),
            last_fm_url: "".to_string(),
            small_image_url,
            medium_image_url,
            large_image_url,
        }
    }
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AlbumList {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub album: Vec<Child>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AlbumList2 {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub album: Vec<AlbumID3>,
}
