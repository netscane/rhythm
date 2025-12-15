use crate::consts;
use error::SubsonicError;

use actix_web::body::BoxBody;
use actix_web::http::header::TryIntoHeaderValue;
use actix_web::http::{header, StatusCode};
use actix_web::Responder;
use serde::Serialize;

pub mod album;
pub mod artist;
pub mod directory;
pub mod error;
pub mod genre;
pub mod lyric;
pub mod music_folder;
pub mod play;
pub mod scan;
pub mod search;
pub mod share;
pub mod song;
pub mod star;
pub mod user;

const VERSION: &str = "1.0";
const STATUS_OK: &str = "ok";
const STATUS_FAILED: &str = "failed";

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct License {
    pub valid: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subsonic {
    pub status: String,
    pub version: String,
    pub r#type: String,
    pub server_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_subsonic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SubsonicError>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_folders: Option<music_folder::MusicFolders>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexes: Option<artist::Indexes>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<directory::Directory>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<user::User>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub users: Option<user::Users>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_list: Option<album::AlbumList>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_list2: Option<album::AlbumList2>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlists: Option<play::Playlists>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist: Option<play::PlaylistWithSongs>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_result: Option<search::SearchResult>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_result2: Option<search::SearchResult2>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_result3: Option<search::SearchResult3>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<star::Starred>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred2: Option<star::Starred2>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub now_playing: Option<play::NowPlaying>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub song: Option<song::Song>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_songs: Option<song::RandomSongs>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub songs_by_genre: Option<song::SongsByGenre>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub genres: Option<genre::Genres>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub artists: Option<artist::Artists>,

    #[serde(rename = "artist", skip_serializing_if = "Option::is_none")]
    pub artist_with_albums_id3: Option<artist::ArtistWithAlbumsID3>,

    #[serde(rename = "album", skip_serializing_if = "Option::is_none")]
    pub album_with_songs_id3: Option<album::AlbumWithSongsID3>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_info: Option<album::AlbumInfo>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_info: Option<artist::ArtistInfo>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_info2: Option<artist::ArtistInfo2>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub similar_songs: Option<song::SimilarSongs>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub similar_songs2: Option<song::SimilarSongs2>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_songs: Option<song::TopSongs>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_queue: Option<play::PlayQueue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub shares: Option<share::Shares>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_status: Option<scan::ScanStatus>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics: Option<lyric::Lyrics>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub jukebox_status: Option<play::JukeboxStatus>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub jukebox_playlist: Option<play::JukeboxPlaylist>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics_list: Option<lyric::LyricsList>,

    /// Subsonic 扩展字段（非标准 API）
    #[serde(flatten)]
    pub ext: SubsonicExt,
}

/// Subsonic 扩展响应字段（非标准 API）
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubsonicExt {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_list: Option<artist::ArtistList>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_list: Option<song::SongList>,
}

impl Default for Subsonic {
    fn default() -> Self {
        Self {
            status: String::from(STATUS_OK),
            version: String::from(VERSION),
            r#type: String::new(),
            server_version: String::from(consts::VERSION),
            open_subsonic: Some(true),
            error: None,
            license: None,
            music_folders: None,
            indexes: None,
            directory: None,
            user: None,
            users: None,
            album_list: None,
            album_list2: None,
            playlists: None,
            playlist: None,
            search_result: None,
            search_result2: None,
            search_result3: None,
            starred: None,
            starred2: None,
            now_playing: None,
            song: None,
            random_songs: None,
            songs_by_genre: None,
            genres: None,
            artists: None,
            artist_with_albums_id3: None,
            album_with_songs_id3: None,
            album_info: None,
            artist_info: None,
            artist_info2: None,
            similar_songs: None,
            similar_songs2: None,
            top_songs: None,
            play_queue: None,
            shares: None,
            scan_status: None,
            lyrics: None,
            jukebox_status: None,
            jukebox_playlist: None,
            lyrics_list: None,
            ext: SubsonicExt::default(),
        }
    }
}

// 宏定义：为 Subsonic 结构体的字段自动生成 From trait 实现
macro_rules! to_subsonic_ok {
    ($field:ident, $type:ty) => {
        impl From<$type> for Subsonic {
            fn from(value: $type) -> Self {
                Self {
                    $field: Some(value),
                    ..Default::default()
                }
            }
        }
    };
}

// 使用宏为各个字段生成 From 实现
to_subsonic_ok!(license, License);
to_subsonic_ok!(music_folders, music_folder::MusicFolders);
to_subsonic_ok!(indexes, artist::Indexes);
to_subsonic_ok!(directory, directory::Directory);
to_subsonic_ok!(user, user::User);
to_subsonic_ok!(users, user::Users);
to_subsonic_ok!(album_list, album::AlbumList);
to_subsonic_ok!(album_list2, album::AlbumList2);
to_subsonic_ok!(playlists, play::Playlists);
to_subsonic_ok!(playlist, play::PlaylistWithSongs);
to_subsonic_ok!(search_result, search::SearchResult);
to_subsonic_ok!(search_result2, search::SearchResult2);
to_subsonic_ok!(search_result3, search::SearchResult3);
to_subsonic_ok!(starred, star::Starred);
to_subsonic_ok!(starred2, star::Starred2);
to_subsonic_ok!(now_playing, play::NowPlaying);
to_subsonic_ok!(song, song::Song);
to_subsonic_ok!(random_songs, song::RandomSongs);
to_subsonic_ok!(songs_by_genre, song::SongsByGenre);
to_subsonic_ok!(genres, genre::Genres);
to_subsonic_ok!(artists, artist::Artists);
to_subsonic_ok!(artist_with_albums_id3, artist::ArtistWithAlbumsID3);
to_subsonic_ok!(album_with_songs_id3, album::AlbumWithSongsID3);
to_subsonic_ok!(album_info, album::AlbumInfo);
to_subsonic_ok!(artist_info, artist::ArtistInfo);
to_subsonic_ok!(artist_info2, artist::ArtistInfo2);
to_subsonic_ok!(similar_songs, song::SimilarSongs);
to_subsonic_ok!(similar_songs2, song::SimilarSongs2);
to_subsonic_ok!(top_songs, song::TopSongs);
to_subsonic_ok!(play_queue, play::PlayQueue);
to_subsonic_ok!(shares, share::Shares);
to_subsonic_ok!(scan_status, scan::ScanStatus);
to_subsonic_ok!(lyrics, lyric::Lyrics);
to_subsonic_ok!(jukebox_status, play::JukeboxStatus);
to_subsonic_ok!(jukebox_playlist, play::JukeboxPlaylist);
to_subsonic_ok!(lyrics_list, lyric::LyricsList);

// 扩展字段的 From 实现
impl From<artist::ArtistList> for Subsonic {
    fn from(value: artist::ArtistList) -> Self {
        Self {
            ext: SubsonicExt {
                artist_list: Some(value),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl From<song::SongList> for Subsonic {
    fn from(value: song::SongList) -> Self {
        Self {
            ext: SubsonicExt {
                song_list: Some(value),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

// 保留原有的错误处理实现
impl From<SubsonicError> for Subsonic {
    fn from(error: SubsonicError) -> Self {
        Self {
            status: String::from(STATUS_FAILED),
            error: Some(error),
            ..Default::default()
        }
    }
}

impl Responder for Subsonic {
    type Body = BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        // 包装为 { "subsonic-response": { ... } } 格式
        let wrapper = JsonWrapper {
            subsonic_response: self,
        };
        let jsonstr = serde_json::to_string(&wrapper).unwrap();
        let mime = mime::APPLICATION_JSON.try_into_value().unwrap();
        let mut res = actix_web::HttpResponse::new(StatusCode::OK);
        res.headers_mut().insert(header::CONTENT_TYPE, mime);
        res.set_body(BoxBody::new(jsonstr))
    }
}

#[derive(Serialize, Debug)]
pub struct JsonWrapper {
    #[serde(rename = "subsonic-response")]
    pub subsonic_response: Subsonic,
}

/*
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiscTitle {
    pub disc: i32,

    pub title: String,
}
    */
