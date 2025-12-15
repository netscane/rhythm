use crate::subsonic::helper::QsQuery;
use crate::subsonic::response::directory::Child;
use crate::subsonic::response::error::SubsonicError;
use crate::subsonic::response::play::{
    Playlist as PlaylistResponse, PlaylistWithSongs, Playlists,
};
use crate::subsonic::response::Subsonic;
use crate::AppState;
use actix_web::{web, HttpMessage, HttpRequest};
use application::command::playlist::{CreatePlaylistCmd, PlaylistAppService, UpdatePlaylistCmd};
use application::query::get_playlist::GetPlaylist;
use infra::repository::postgres::command::playlist::PlaylistRepositoryImpl;
use infra::repository::postgres::query::playlist::PlaylistDaoImpl;
use model::playlist::{Playlist, PlaylistAudioFile, PlaylistSummary};
use serde::Deserialize;
use std::sync::Arc;

/// createPlaylist API 请求参数
///
/// 根据 OpenSubsonic 规范:
/// - playlistId: 更新时必需，播放列表 ID
/// - name: 创建时必需，播放列表名称
/// - songId: 可选，播放列表中的歌曲 ID，可以有多个
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlaylistQuery {
    /// 播放列表 ID（更新时必需）
    #[serde(default)]
    pub playlist_id: Option<String>,

    /// 播放列表名称（创建时必需）
    #[serde(default)]
    pub name: Option<String>,

    /// 歌曲 ID 列表
    #[serde(default)]
    pub song_id: Vec<String>,
}

/// createPlaylist - 创建或更新播放列表
///
/// 根据 OpenSubsonic 规范 (Since 1.2.0):
/// - 如果提供 playlistId，则更新现有播放列表
/// - 如果提供 name（且无 playlistId），则创建新播放列表
/// - songId 参数可以多次出现，用于指定播放列表中的歌曲
pub async fn create_playlist(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: QsQuery<CreatePlaylistQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取用户
    let user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 创建 Command 服务
    let playlist_repo: Arc<dyn domain::playlist::PlaylistRepository> =
        Arc::new(PlaylistRepositoryImpl::new(state.db.clone()));
    let playlist_app_service = PlaylistAppService::new(playlist_repo, state.id_generator.clone());

    // 创建 Query 服务
    let playlist_dao = Arc::new(PlaylistDaoImpl::new(state.db.clone()));
    let get_playlist = GetPlaylist::new(playlist_dao);

    // 解析参数
    let playlist_id: Option<i64> = query
        .playlist_id
        .as_ref()
        .and_then(|id| id.parse::<i64>().ok());

    let song_ids: Vec<i64> = query
        .song_id
        .iter()
        .filter_map(|id| id.parse::<i64>().ok())
        .collect();

    // 调用应用服务创建或更新播放列表
    let playlist = playlist_app_service
        .create_playlist(CreatePlaylistCmd {
            playlist_id,
            name: query.name.clone(),
            owner_id: user.id.as_i64(),
            owner_name: user.name.clone(),
            song_ids,
        })
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    // 通过 Query 服务获取完整的播放列表信息
    let playlist_detail = get_playlist
        .get_by_id(playlist.id.as_i64())
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    // 构建响应
    let response = PlaylistWithSongs {
        playlist: PlaylistResponse {
            id: playlist_detail.id.to_string(),
            name: playlist_detail.name,
            comment: playlist_detail.comment,
            owner: Some(playlist_detail.owner_name),
            public: Some(playlist_detail.public),
            song_count: playlist_detail.song_count,
            duration: playlist_detail.duration,
            created: format_timestamp(playlist_detail.created_at),
            changed: format_timestamp(playlist_detail.updated_at),
            cover_art: None,
            allowed_user: None,
        },
        entry: None, // 暂不返回歌曲详情，可根据需要添加
    };

    Ok(response.into())
}

/// 格式化时间戳为 ISO 8601 格式
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        .unwrap_or_default()
}

/// getPlaylists API 请求参数
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPlaylistsQuery {
    /// 用户名（需要 admin 权限）
    #[serde(default)]
    pub username: Option<String>,
}

/// getPlaylists - 获取用户的所有播放列表
///
/// 根据 OpenSubsonic 规范 (Since 1.0.0):
/// - 返回用户有权播放的所有播放列表
/// - 如果指定 username 参数，需要 admin 权限
pub async fn get_playlists(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<GetPlaylistsQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取用户
    let user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 确定要查询的用户 ID
    let target_owner_id = if let Some(ref username) = query.username {
        // 需要 admin 权限
        if !user.is_admin {
            return Err(
                SubsonicError::error_authorization_fail().wrap("Admin role required".to_string())
            );
        }
        // TODO: 根据 username 查找用户 ID，暂时返回错误
        return Err(
            SubsonicError::error_generic().wrap("Username lookup not implemented".to_string())
        );
    } else {
        user.id.as_i64()
    };

    // 创建 Query 服务
    let playlist_dao = Arc::new(PlaylistDaoImpl::new(state.db.clone()));
    let get_playlist = GetPlaylist::new(playlist_dao);

    // 获取播放列表
    let playlists = get_playlist
        .get_by_owner_id(target_owner_id)
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    // 转换为响应格式
    let playlist_responses: Vec<PlaylistResponse> =
        playlists.into_iter().map(playlist_to_response).collect();

    let response = Playlists {
        playlist: if playlist_responses.is_empty() {
            None
        } else {
            Some(playlist_responses)
        },
    };

    Ok(response.into())
}

/// 将 PlaylistSummary 转换为 API 响应
fn playlist_to_response(p: PlaylistSummary) -> PlaylistResponse {
    PlaylistResponse {
        id: p.id.to_string(),
        name: p.name,
        comment: p.comment,
        owner: Some(p.owner_name),
        public: Some(p.public),
        song_count: p.song_count,
        duration: p.duration,
        created: format_timestamp(p.created_at),
        changed: format_timestamp(p.updated_at),
        cover_art: None,
        allowed_user: None,
    }
}

/// getPlaylist API 请求参数
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPlaylistQuery {
    /// 播放列表 ID（必需）
    pub id: String,
}

/// getPlaylist - 获取播放列表详情（包含歌曲列表）
///
/// 根据 OpenSubsonic 规范 (Since 1.0.0):
/// - 返回播放列表中的文件列表
pub async fn get_playlist(
    state: web::Data<AppState>,
    _req: HttpRequest,
    query: web::Query<GetPlaylistQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 解析播放列表 ID
    let playlist_id: i64 = query
        .id
        .parse()
        .map_err(|_| SubsonicError::error_generic().wrap("Invalid playlist ID".to_string()))?;

    // 创建 Query 服务
    let playlist_dao = Arc::new(PlaylistDaoImpl::new(state.db.clone()));
    let get_playlist_svc = GetPlaylist::new(playlist_dao);

    // 获取播放列表详情
    let playlist = get_playlist_svc
        .get_by_id(playlist_id)
        .await
        .map_err(|e| SubsonicError::error_data_not_found().wrap(e.to_string()))?;

    // 转换为响应格式
    let response = playlist_detail_to_response(playlist);

    Ok(response.into())
}

/// 将 Playlist 转换为 PlaylistWithSongs 响应
fn playlist_detail_to_response(p: Playlist) -> PlaylistWithSongs {
    let entries: Vec<Child> = p
        .tracks
        .iter()
        .map(|t| audio_file_to_child(&t.audio_file))
        .collect();

    PlaylistWithSongs {
        playlist: PlaylistResponse {
            id: p.id.to_string(),
            name: p.name,
            comment: p.comment,
            owner: Some(p.owner_name),
            public: Some(p.public),
            song_count: p.song_count,
            duration: p.duration,
            created: format_timestamp(p.created_at),
            changed: format_timestamp(p.updated_at),
            cover_art: None,
            allowed_user: None,
        },
        entry: if entries.is_empty() {
            None
        } else {
            Some(entries)
        },
    }
}

/// 将 PlaylistAudioFile 转换为 Child
fn audio_file_to_child(af: &PlaylistAudioFile) -> Child {
    use application::query::dto::cover_art::audio_file_cover_art_id;

    Child {
        id: af.id.to_string(),
        parent: af.album_id.map(|id| id.to_string()),
        is_dir: false,
        title: af.title.clone(),
        name: af.title.clone(),
        album: af.album_name.clone(),
        artist: af.artist_name.clone(),
        track: af.track_number,
        year: af.year,
        genre: af.genre.clone(),
        cover_art: af.cover_art_id.map(|_| audio_file_cover_art_id(af.id)),
        size: af.size,
        content_type: af.content_type.clone(),
        suffix: af.suffix.clone(),
        starred: None, // TODO: 转换 starred 时间
        transcoded_content_type: None,
        transcoded_suffix: None,
        duration: af.duration.map(|d| d as i64).unwrap_or(0),
        bit_rate: af.bit_rate,
        path: Some(af.path.clone()),
        play_count: af.play_count,
        disk_number: af.disc_number.unwrap_or(0),
        created: None, // TODO: 转换 created_at
        album_id: af.album_id.map(|id| id.to_string()),
        artist_id: af.artist_id.map(|id| id.to_string()),
        r#type: Some("music".to_string()),
        user_rating: af.rating,
        song_count: 0,
        is_video: false,
    }
}

/// deletePlaylist API 请求参数
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePlaylistQuery {
    /// 播放列表 ID（必需）
    pub id: String,
}

/// deletePlaylist - 删除播放列表
///
/// 根据 OpenSubsonic 规范 (Since 1.2.0):
/// - 删除指定的播放列表
pub async fn delete_playlist(
    state: web::Data<AppState>,
    _req: HttpRequest,
    query: web::Query<DeletePlaylistQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 解析播放列表 ID
    let playlist_id: i64 = query
        .id
        .parse()
        .map_err(|_| SubsonicError::error_generic().wrap("Invalid playlist ID".to_string()))?;

    // 创建 Command 服务
    let playlist_repo: Arc<dyn domain::playlist::PlaylistRepository> =
        Arc::new(PlaylistRepositoryImpl::new(state.db.clone()));
    let playlist_app_service = PlaylistAppService::new(playlist_repo, state.id_generator.clone());

    // 删除播放列表
    playlist_app_service
        .delete_playlist(playlist_id)
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    Ok(Subsonic::default())
}

/// updatePlaylist API 请求参数
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlaylistQuery {
    /// 播放列表 ID（必需）
    pub playlist_id: String,

    /// 播放列表名称
    #[serde(default)]
    pub name: Option<String>,

    /// 播放列表备注
    #[serde(default)]
    pub comment: Option<String>,

    /// 是否公开
    #[serde(default)]
    pub public: Option<bool>,

    /// 要添加的歌曲 ID 列表
    #[serde(default)]
    pub song_id_to_add: Vec<String>,

    /// 要删除的歌曲索引列表
    #[serde(default)]
    pub song_index_to_remove: Vec<usize>,
}

/// updatePlaylist - 更新播放列表
///
/// 根据 OpenSubsonic 规范 (Since 1.8.0):
/// - 只有播放列表的所有者可以更新它
pub async fn update_playlist(
    state: web::Data<AppState>,
    _req: HttpRequest,
    query: QsQuery<UpdatePlaylistQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 解析播放列表 ID
    let playlist_id: i64 = query
        .playlist_id
        .parse()
        .map_err(|_| SubsonicError::error_generic().wrap("Invalid playlist ID".to_string()))?;

    // 解析要添加的歌曲 ID
    let song_ids_to_add: Vec<i64> = query
        .song_id_to_add
        .iter()
        .filter_map(|id| id.parse::<i64>().ok())
        .collect();

    // 创建 Command 服务
    let playlist_repo: Arc<dyn domain::playlist::PlaylistRepository> =
        Arc::new(PlaylistRepositoryImpl::new(state.db.clone()));
    let playlist_app_service = PlaylistAppService::new(playlist_repo, state.id_generator.clone());

    // 更新播放列表
    playlist_app_service
        .update_playlist(UpdatePlaylistCmd {
            playlist_id,
            name: query.name.clone(),
            comment: query.comment.clone(),
            public: query.public,
            song_ids_to_add,
            song_indexes_to_remove: query.song_index_to_remove.clone(),
        })
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    Ok(Subsonic::default())
}
