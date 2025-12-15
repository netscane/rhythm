use crate::subsonic::response::error::SubsonicError;
use crate::subsonic::response::Subsonic;
use crate::AppState;
use actix_web::{web, HttpMessage, HttpRequest};
use application::command::media_annotation::{
    MediaAnnotationService, ScrobbleCmd, SetRatingCmd, StarCmd, StarItem, UnstarCmd, UnstarItem,
};
use application::context::AppContext;
use domain::value::{AudioFileId, PlayerId};
use infra::repository::postgres::command::album::AlbumRepositoryImpl;
use infra::repository::postgres::command::annotation::AnnotationRepositoryImpl;
use infra::repository::postgres::command::artist::ArtistRepositoryImpl;
use infra::repository::postgres::command::audio_file::AudioFileRepositoryImpl;
use infra::repository::postgres::command::player::PlayerRepositoryImpl;
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StarQuery {
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub id: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub album_id: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub artist_id: Vec<String>,
}

pub async fn star(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<StarQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取用户
    let user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 构建 items 列表 - 合并 id, albumId, artistId
    let mut items: Vec<StarItem> = Vec::new();
    
    // 添加 song ids
    for id in &query.id {
        if let Ok(item_id) = id.parse::<i64>() {
            items.push(StarItem { item_id });
        }
    }
    
    // 添加 album ids
    for id in &query.album_id {
        if let Ok(item_id) = id.parse::<i64>() {
            items.push(StarItem { item_id });
        }
    }
    
    // 添加 artist ids
    for id in &query.artist_id {
        if let Ok(item_id) = id.parse::<i64>() {
            items.push(StarItem { item_id });
        }
    }

    // 创建仓储和服务
    let annotation_repo = Arc::new(AnnotationRepositoryImpl::new(state.db.clone()));
    let audio_file_repo = Arc::new(AudioFileRepositoryImpl::new(state.db.clone()));
    let album_repo = Arc::new(AlbumRepositoryImpl::new(
        state.db.clone(),
        state.id_generator.clone(),
    ));
    let artist_repo = Arc::new(ArtistRepositoryImpl::new(
        state.db.clone(),
        state.id_generator.clone(),
    ));
    let player_repo = Arc::new(PlayerRepositoryImpl::new(state.db.clone()));
    let event_bus = Arc::new(state.event_bus.clone());
    let id_generator = state.id_generator.clone();

    let svc = MediaAnnotationService::new(
        annotation_repo,
        audio_file_repo,
        album_repo,
        artist_repo,
        player_repo,
        id_generator,
        event_bus,
    );

    let ctx = AppContext::new();
    match svc
        .star(
            &ctx,
            StarCmd {
                user_id: user.id.clone(),
                items,
            },
        )
        .await
    {
        Ok(_) => Ok(Subsonic::default()),
        Err(e) => Err(SubsonicError::error_generic().wrap(e.to_string())),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnstarQuery {
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub id: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub album_id: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub artist_id: Vec<String>,
}

pub async fn unstar(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<UnstarQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取用户
    let user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 构建 items 列表 - 合并 id, albumId, artistId
    let mut items: Vec<UnstarItem> = Vec::new();
    
    // 添加 song ids
    for id in &query.id {
        if let Ok(item_id) = id.parse::<i64>() {
            items.push(UnstarItem { item_id });
        }
    }
    
    // 添加 album ids
    for id in &query.album_id {
        if let Ok(item_id) = id.parse::<i64>() {
            items.push(UnstarItem { item_id });
        }
    }
    
    // 添加 artist ids
    for id in &query.artist_id {
        if let Ok(item_id) = id.parse::<i64>() {
            items.push(UnstarItem { item_id });
        }
    }

    // 创建仓储和服务
    let annotation_repo = Arc::new(AnnotationRepositoryImpl::new(state.db.clone()));
    let audio_file_repo = Arc::new(AudioFileRepositoryImpl::new(state.db.clone()));
    let album_repo = Arc::new(AlbumRepositoryImpl::new(
        state.db.clone(),
        state.id_generator.clone(),
    ));
    let artist_repo = Arc::new(ArtistRepositoryImpl::new(
        state.db.clone(),
        state.id_generator.clone(),
    ));
    let player_repo = Arc::new(PlayerRepositoryImpl::new(state.db.clone()));
    let event_bus = Arc::new(state.event_bus.clone());
    let id_generator = state.id_generator.clone();

    let svc = MediaAnnotationService::new(
        annotation_repo,
        audio_file_repo,
        album_repo,
        artist_repo,
        player_repo,
        id_generator,
        event_bus,
    );

    let ctx = AppContext::new();
    match svc
        .unstar(
            &ctx,
            UnstarCmd {
                user_id: user.id.clone(),
                items,
            },
        )
        .await
    {
        Ok(_) => Ok(Subsonic::default()),
        Err(e) => Err(SubsonicError::error_generic().wrap(e.to_string())),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRatingQuery {
    pub id: String,
    pub rating: i32,
}

pub async fn set_rating(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<SetRatingQuery>,
) -> Result<Subsonic, SubsonicError> {
    // 从 request extensions 中获取用户
    let user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 解析 id
    let item_id: i64 = query
        .id
        .parse()
        .map_err(|_| SubsonicError::error_generic().wrap("Invalid id format".to_string()))?;

    // 创建仓储和服务
    let annotation_repo = Arc::new(AnnotationRepositoryImpl::new(state.db.clone()));
    let audio_file_repo = Arc::new(AudioFileRepositoryImpl::new(state.db.clone()));
    let album_repo = Arc::new(AlbumRepositoryImpl::new(
        state.db.clone(),
        state.id_generator.clone(),
    ));
    let artist_repo = Arc::new(ArtistRepositoryImpl::new(
        state.db.clone(),
        state.id_generator.clone(),
    ));
    let player_repo = Arc::new(PlayerRepositoryImpl::new(state.db.clone()));
    let event_bus = Arc::new(state.event_bus.clone());
    let id_generator = state.id_generator.clone();

    let svc = MediaAnnotationService::new(
        annotation_repo,
        audio_file_repo,
        album_repo,
        artist_repo,
        player_repo,
        id_generator,
        event_bus,
    );

    let ctx = AppContext::new();
    match svc
        .set_rating(
            &ctx,
            SetRatingCmd {
                user_id: user.id.clone(),
                item_id,
                rating: query.rating,
            },
        )
        .await
    {
        Ok(_) => Ok(Subsonic::default()),
        Err(e) => Err(SubsonicError::error_generic().wrap(e.to_string())),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrobbleQuery {
    pub id: String,
    #[serde(default)]
    pub time: Option<i64>,
    #[serde(default = "default_submission")]
    pub submission: bool,
}

fn default_submission() -> bool {
    true
}

pub async fn scrobble(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ScrobbleQuery>,
) -> Result<Subsonic, SubsonicError> {
    use crate::middleware::other::{ClientUniqueID, RequestClient};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use url::Url;

    // 解析 id
    let audio_file_id: i64 = query
        .id
        .parse()
        .map_err(|_| SubsonicError::error_generic().wrap("Invalid id format".to_string()))?;

    // 从 request extensions 中获取用户
    let user = req
        .extensions()
        .get::<domain::user::User>()
        .ok_or_else(|| SubsonicError::error_generic().wrap("User not found".to_string()))?
        .clone();

    // 从 request extensions 中获取 client（由中间件设置）
    // 如果没有，从查询参数中获取，最后才使用 User-Agent 作为后备
    let client = req
        .extensions()
        .get::<RequestClient>()
        .map(|rc| rc.0.clone())
        .or_else(|| {
            // 从查询参数中获取
            let query_string = req.query_string();
            let url = format!("http://localhost/?{}", query_string);
            Url::parse(&url)
                .ok()?
                .query_pairs()
                .find(|(key, _)| key == "c")
                .map(|(_, value)| value.to_string())
        })
        .unwrap_or_else(|| {
            // 最后使用 User-Agent 作为后备
            req.headers()
                .get("User-Agent")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("unknown")
                .to_string()
        });

    // 从 request 中获取 IP 地址
    let ip = req
        .connection_info()
        .peer_addr()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // 从 User-Agent header 获取 user_agent
    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // 创建仓储和服务
    let annotation_repo = Arc::new(AnnotationRepositoryImpl::new(state.db.clone()));
    let audio_file_repo = Arc::new(AudioFileRepositoryImpl::new(state.db.clone()));
    let album_repo = Arc::new(AlbumRepositoryImpl::new(
        state.db.clone(),
        state.id_generator.clone(),
    ));
    let artist_repo = Arc::new(ArtistRepositoryImpl::new(
        state.db.clone(),
        state.id_generator.clone(),
    ));
    let player_repo = Arc::new(PlayerRepositoryImpl::new(state.db.clone()));
    let event_bus = Arc::new(state.event_bus.clone());
    let id_generator = state.id_generator.clone();

    let svc = MediaAnnotationService::new(
        annotation_repo,
        audio_file_repo,
        album_repo,
        artist_repo,
        player_repo,
        id_generator,
        event_bus,
    );

    // 从客户端生成的 client_id (UUID 字符串) 生成 player_id
    // client_id 由客户端生成并通过 client_unique_id 中间件设置到 request extensions
    // 使用 client_id 的 hash 值生成稳定的 player_id，确保同一客户端总是得到相同的 player_id
    let player_id = req
        .extensions()
        .get::<ClientUniqueID>()
        .map(|cid| {
            // client_id 是客户端生成的 UUID 字符串，使用 hash 值生成 player_id
            // 这样可以确保相同的 client_id 总是生成相同的 player_id
            let mut hasher = DefaultHasher::new();
            cid.0.hash(&mut hasher);
            let hash = hasher.finish() as i64;
            PlayerId::from(hash)
        })
        .unwrap_or_else(|| {
            // 如果没有 ClientUniqueID（客户端未提供），使用用户ID作为后备
            // 注意：这种情况下，应用服务可能会创建新的 player
            PlayerId::from(user.id.as_i64())
        });

    let ctx = AppContext::new();
    match svc
        .scrobble(
            &ctx,
            ScrobbleCmd {
                player_id,
                client,
                ip,
                user_agent,
                user_id: user.id.clone(),
                audio_file_id: AudioFileId::from(audio_file_id),
                submission: query.submission,
            },
        )
        .await
    {
        Ok(_) => Ok(Subsonic::default()),
        Err(e) => Err(SubsonicError::error_generic().wrap(e.to_string())),
    }
}
