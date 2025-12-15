use crate::subsonic::response::directory::Child;
use crate::subsonic::response::error::SubsonicError;
use crate::subsonic::response::play::PlayQueue as PlayQueueResponse;
use crate::subsonic::response::Subsonic;
use crate::AppState;
use actix_web::{web, HttpMessage, HttpRequest};
use application::command::play_queue::{PlayQueueAppService, SavePlayQueueCmd};
use application::query::get_play_queue::GetPlayQueue;
use infra::repository::postgres::command::play_queue::PlayQueueRepositoryImpl;
use infra::repository::postgres::query::play_queue::PlayQueueDaoImpl;
use model::playlist::PlaylistAudioFile;
use serde::Deserialize;
use std::sync::Arc;

/// 自定义反序列化：支持单个值或多个值
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, SeqAccess, Visitor};
    use std::fmt;

    struct StringOrVec;

    impl<'de> Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or a sequence of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_string()])
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(value) = seq.next_element::<String>()? {
                vec.push(value);
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

/// savePlayQueue API 请求参数
///
/// 根据 OpenSubsonic 规范 (Since 1.12.0):
/// - id: 播放队列中的歌曲 ID，可以有多个
/// - current: 当前播放的歌曲 ID
/// - position: 当前歌曲的播放位置（毫秒）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePlayQueueQuery {
    /// 歌曲 ID 列表
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub id: Vec<String>,

    /// 当前播放的歌曲 ID
    #[serde(default)]
    pub current: Option<String>,

    /// 当前歌曲的播放位置（毫秒）
    #[serde(default)]
    pub position: Option<i64>,
}

/// savePlayQueue - 保存播放队列状态
///
/// 根据 OpenSubsonic 规范 (Since 1.12.0):
/// - 保存用户的播放队列状态，包括队列中的歌曲、当前播放的歌曲和播放位置
/// - 通常用于在不同客户端之间同步播放状态（例如听有声书时）
///
/// OpenSubsonic 扩展:
/// - 如果不传任何参数，则清空当前保存的队列
/// - 如果 id 为空，current 也不是必需的
/// - 如果 position 为空，服务器应将位置视为 0
pub async fn save_play_queue(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<SavePlayQueueQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取用户
    let user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 解析歌曲 ID 列表
    let song_ids: Vec<i64> = query
        .id
        .iter()
        .filter_map(|id| id.parse::<i64>().ok())
        .collect();

    // 解析当前播放的歌曲 ID
    let current_id: Option<i64> = query.current.as_ref().and_then(|id| id.parse::<i64>().ok());

    // 获取播放位置，默认为 0
    let position = query.position.unwrap_or(0);

    // 获取客户端名称
    let changed_by = req
        .headers()
        .get("X-Subsonic-Client")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // 创建 Command 服务
    let play_queue_repo: Arc<dyn domain::play_queue::PlayQueueRepository> =
        Arc::new(PlayQueueRepositoryImpl::new(state.db.clone()));
    let play_queue_app_service =
        PlayQueueAppService::new(play_queue_repo, state.id_generator.clone());

    // 保存播放队列
    play_queue_app_service
        .save_play_queue(SavePlayQueueCmd {
            user_id: user.id.as_i64(),
            song_ids,
            current_id,
            position,
            changed_by,
        })
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    Ok(Subsonic::default())
}

/// getPlayQueue - 获取播放队列状态
///
/// 根据 OpenSubsonic 规范 (Since 1.12.0):
/// - 返回用户的播放队列状态（由 savePlayQueue 设置）
/// - 包括队列中的歌曲、当前播放的歌曲和播放位置
/// - 通常用于在不同客户端之间同步播放状态
pub async fn get_play_queue(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取用户
    let user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 创建 Query 服务
    let play_queue_dao = Arc::new(PlayQueueDaoImpl::new(state.db.clone()));
    let get_play_queue_svc = GetPlayQueue::new(play_queue_dao);

    // 获取播放队列
    let play_queue = get_play_queue_svc
        .get_by_user_id(user.id.as_i64(), &user.name)
        .await
        .map_err(|e| SubsonicError::error_generic().wrap(e.to_string()))?;

    match play_queue {
        Some(pq) => {
            let entries: Vec<Child> = pq.entries.iter().map(audio_file_to_child).collect();

            let response = PlayQueueResponse {
                entry: if entries.is_empty() {
                    None
                } else {
                    Some(entries)
                },
                current: pq.current_id.map(|id| id.to_string()),
                position: Some(pq.position),
                username: pq.username,
                changed: Some(pq.changed),
                changed_by: pq.changed_by,
            };

            Ok(response.into())
        }
        None => {
            // 返回空的播放队列
            let response = PlayQueueResponse {
                entry: None,
                current: None,
                position: None,
                username: user.name,
                changed: None,
                changed_by: String::new(),
            };

            Ok(response.into())
        }
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
