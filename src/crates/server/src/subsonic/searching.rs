use crate::subsonic::response::album::AlbumID3;
use crate::subsonic::response::artist::{Artist as ArtistResponse, ArtistID3};
use crate::subsonic::response::directory::Child;
use crate::subsonic::response::error::SubsonicError;
use crate::subsonic::response::search::{SearchResult, SearchResult2, SearchResult3};
use crate::subsonic::response::Subsonic;
use crate::AppState;
use actix_web::web;
use application::query::dao::{AlbumDao, ArtistDao, AudioFileDao};
use infra::repository::postgres::query::album::AlbumDaoImpl;
use infra::repository::postgres::query::artist::ArtistDaoImpl;
use infra::repository::postgres::query::audio_file::AudioFileDaoImpl;
use serde::Deserialize;

/// search API 请求参数
///
/// 根据 Subsonic 规范 (Since 1.0.0, Deprecated since 1.4.0):
/// - artist: 按艺术家搜索（可选）
/// - album: 按专辑搜索（可选）
/// - title: 按歌曲标题搜索（可选）
/// - any: 搜索所有字段（可选）
/// - count: 最大返回结果数（可选，默认 20）
/// - offset: 结果偏移量，用于分页（可选，默认 0）
/// - newerThan: 只返回比此时间更新的结果，毫秒时间戳（可选）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    /// 按艺术家搜索
    #[serde(default)]
    pub artist: Option<String>,

    /// 按专辑搜索
    #[serde(default)]
    pub album: Option<String>,

    /// 按歌曲标题搜索
    #[serde(default)]
    pub title: Option<String>,

    /// 搜索所有字段
    #[serde(default)]
    pub any: Option<String>,

    /// 最大返回结果数
    #[serde(default = "default_count")]
    pub count: i32,

    /// 结果偏移量
    #[serde(default)]
    pub offset: i32,

    /// 只返回比此时间更新的结果（毫秒时间戳）
    #[serde(default)]
    pub newer_than: Option<i64>,
}

fn default_count() -> i32 {
    20
}

/// search - 搜索文件
///
/// 根据 Subsonic 规范 (Since 1.0.0, Deprecated since 1.4.0):
/// - 返回匹配搜索条件的文件列表
/// - 支持分页
/// - 已弃用，推荐使用 search2
pub async fn search(
    state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> Result<Subsonic, SubsonicError> {
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());

    // 执行搜索
    let (audio_files, total) = audio_file_dao
        .search(
            query.any.as_deref(),
            query.artist.as_deref(),
            query.album.as_deref(),
            query.title.as_deref(),
            query.newer_than,
            query.offset,
            query.count,
        )
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    // 转换为 Child 列表
    let matches: Vec<Child> = audio_files.into_iter().map(Child::from).collect();

    let search_result = SearchResult {
        total_hits: Some(total),
        offset: Some(query.offset),
        r#match: if matches.is_empty() {
            None
        } else {
            Some(matches)
        },
    };

    Ok(search_result.into())
}

/// search2 API 请求参数
///
/// 根据 Subsonic 规范 (Since 1.4.0):
/// - query: 搜索查询（必填）
/// - artistCount: 最大返回艺术家数（可选，默认 20）
/// - artistOffset: 艺术家结果偏移量（可选，默认 0）
/// - albumCount: 最大返回专辑数（可选，默认 20）
/// - albumOffset: 专辑结果偏移量（可选，默认 0）
/// - songCount: 最大返回歌曲数（可选，默认 20）
/// - songOffset: 歌曲结果偏移量（可选，默认 0）
/// - musicFolderId: 音乐文件夹 ID（可选，Since 1.12.0）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Search2Query {
    /// 搜索查询
    pub query: String,

    /// 最大返回艺术家数
    #[serde(default = "default_count")]
    pub artist_count: i32,

    /// 艺术家结果偏移量
    #[serde(default)]
    pub artist_offset: i32,

    /// 最大返回专辑数
    #[serde(default = "default_count")]
    pub album_count: i32,

    /// 专辑结果偏移量
    #[serde(default)]
    pub album_offset: i32,

    /// 最大返回歌曲数
    #[serde(default = "default_count")]
    pub song_count: i32,

    /// 歌曲结果偏移量
    #[serde(default)]
    pub song_offset: i32,

    /// 音乐文件夹 ID（Since 1.12.0）
    #[serde(default)]
    pub music_folder_id: Option<i64>,
}

/// search2 - 搜索艺术家、专辑和歌曲
///
/// 根据 Subsonic 规范 (Since 1.4.0):
/// - 返回匹配搜索条件的艺术家、专辑和歌曲列表
/// - 支持分页
pub async fn search2(
    state: web::Data<AppState>,
    query: web::Query<Search2Query>,
) -> Result<Subsonic, SubsonicError> {
    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let album_dao = AlbumDaoImpl::new(state.db.clone());
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());

    // 并行执行三个搜索
    let (artists_result, albums_result, songs_result) = tokio::join!(
        artist_dao.search(&query.query, query.artist_offset, query.artist_count),
        album_dao.search(&query.query, query.album_offset, query.album_count),
        audio_file_dao.search(
            Some(&query.query),
            None,
            None,
            None,
            None,
            query.song_offset,
            query.song_count
        )
    );

    // 处理艺术家结果
    let artists = artists_result
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;
    let artist_responses: Vec<ArtistResponse> = artists
        .into_iter()
        .map(|artist| {
            let cover_art = format!("ar-{}", artist.id);
            ArtistResponse {
                id: artist.id.to_string(),
                name: artist.name,
                starred: artist.starred_at,
                user_rating: artist.rating,
                cover_art,
                artist_image_url: String::new(),
            }
        })
        .collect();

    // 处理专辑结果
    let albums = albums_result
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;
    let album_responses: Vec<Child> = albums.into_iter().map(Child::from).collect();

    // 处理歌曲结果
    let (songs, _total) = songs_result
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;
    let song_responses: Vec<Child> = songs.into_iter().map(Child::from).collect();

    let search_result = SearchResult2 {
        artist: if artist_responses.is_empty() {
            None
        } else {
            Some(artist_responses)
        },
        album: if album_responses.is_empty() {
            None
        } else {
            Some(album_responses)
        },
        song: if song_responses.is_empty() {
            None
        } else {
            Some(song_responses)
        },
    };

    Ok(search_result.into())
}

/// search3 API 请求参数
///
/// 根据 Subsonic 规范 (Since 1.8.0):
/// - query: 搜索查询（必填，OpenSubsonic 支持空查询返回所有数据）
/// - artistCount: 最大返回艺术家数（可选，默认 20）
/// - artistOffset: 艺术家结果偏移量（可选，默认 0）
/// - albumCount: 最大返回专辑数（可选，默认 20）
/// - albumOffset: 专辑结果偏移量（可选，默认 0）
/// - songCount: 最大返回歌曲数（可选，默认 20）
/// - songOffset: 歌曲结果偏移量（可选，默认 0）
/// - musicFolderId: 音乐文件夹 ID（可选，Since 1.12.0）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Search3Query {
    /// 搜索查询（OpenSubsonic 支持空查询）
    #[serde(default)]
    pub query: String,

    /// 最大返回艺术家数
    #[serde(default = "default_count")]
    pub artist_count: i32,

    /// 艺术家结果偏移量
    #[serde(default)]
    pub artist_offset: i32,

    /// 最大返回专辑数
    #[serde(default = "default_count")]
    pub album_count: i32,

    /// 专辑结果偏移量
    #[serde(default)]
    pub album_offset: i32,

    /// 最大返回歌曲数
    #[serde(default = "default_count")]
    pub song_count: i32,

    /// 歌曲结果偏移量
    #[serde(default)]
    pub song_offset: i32,

    /// 音乐文件夹 ID（Since 1.12.0）
    #[serde(default)]
    pub music_folder_id: Option<i64>,
}

/// search3 - 搜索艺术家、专辑和歌曲（ID3 标签）
///
/// 根据 Subsonic 规范 (Since 1.8.0):
/// - 返回匹配搜索条件的艺术家、专辑和歌曲列表
/// - 音乐按 ID3 标签组织
/// - 支持分页
/// - OpenSubsonic: 支持空查询返回所有数据用于离线同步
pub async fn search3(
    state: web::Data<AppState>,
    query: web::Query<Search3Query>,
) -> Result<Subsonic, SubsonicError> {
    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let album_dao = AlbumDaoImpl::new(state.db.clone());
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());

    // 处理空查询 - OpenSubsonic 要求支持空查询返回所有数据
    let search_query = if query.query.is_empty() || query.query == "\"\"" {
        None
    } else {
        Some(query.query.as_str())
    };

    // 并行执行三个搜索
    let (artists_result, albums_result, songs_result) = tokio::join!(
        artist_dao.search(
            search_query.unwrap_or(""),
            query.artist_offset,
            query.artist_count
        ),
        album_dao.search(
            search_query.unwrap_or(""),
            query.album_offset,
            query.album_count
        ),
        audio_file_dao.search(
            search_query,
            None,
            None,
            None,
            None,
            query.song_offset,
            query.song_count
        )
    );

    // 处理艺术家结果 - 使用 ArtistID3 格式
    let artists = artists_result
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;
    let artist_responses: Vec<ArtistID3> = artists
        .into_iter()
        .map(|artist| {
            let cover_art = format!("ar-{}", artist.id);
            ArtistID3::new(artist, cover_art, String::new())
        })
        .collect();

    // 处理专辑结果 - 使用 AlbumID3 格式
    let albums = albums_result
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;
    let album_responses: Vec<AlbumID3> = albums.into_iter().map(AlbumID3::new).collect();

    // 处理歌曲结果
    let (songs, _total) = songs_result
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;
    let song_responses: Vec<Child> = songs.into_iter().map(Child::from).collect();

    let search_result = SearchResult3 {
        artist: if artist_responses.is_empty() {
            None
        } else {
            Some(artist_responses)
        },
        album: if album_responses.is_empty() {
            None
        } else {
            Some(album_responses)
        },
        song: if song_responses.is_empty() {
            None
        } else {
            Some(song_responses)
        },
    };

    Ok(search_result.into())
}
