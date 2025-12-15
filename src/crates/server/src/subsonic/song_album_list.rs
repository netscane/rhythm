use crate::subsonic::response::album::{AlbumID3, AlbumList, AlbumList2};
use crate::subsonic::response::artist::{Artist, ArtistID3, ArtistList};
use crate::subsonic::response::directory::Child;
use crate::subsonic::response::error::SubsonicError;
use crate::subsonic::response::song::{RandomSongs, SongList, Songs, SongsByGenre};
use crate::subsonic::response::star::{Starred, Starred2};
use crate::subsonic::response::{JsonWrapper, Subsonic};
use crate::AppState;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use application::query::get_album_list::GetAlbumList;
use application::query::get_artist_list::GetArtistList;
use application::query::get_random_songs::GetRandomSongs;
use application::query::get_songs_by_genre::GetSongsByGenre;
use application::query::get_songs_list::GetSongsList;
use application::query::get_starred::GetStarred;
use infra::auth::{AuthConfig, JwtTokenService};
use infra::repository::postgres::query::album::AlbumDaoImpl;
use infra::repository::postgres::query::artist::ArtistDaoImpl;
use infra::repository::postgres::query::audio_file::AudioFileDaoImpl;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAlbumListQuery {
    pub r#type: String,
    pub genre: Option<String>,
    pub from_year: Option<i32>,
    pub to_year: Option<i32>,
    pub offset: Option<i32>,
    pub size: Option<i32>,
}

pub async fn get_album_list(
    state: web::Data<AppState>,
    query: web::Query<GetAlbumListQuery>,
) -> impl Responder {
    let album_dao = AlbumDaoImpl::new(state.db.clone());
    let usecase = GetAlbumList::new(Arc::new(album_dao));

    let offset = query.offset.unwrap_or(0);
    let size = query.size.unwrap_or(10);

    let (albums, count) = match usecase
        .handle(
            &query.r#type,
            query.genre.as_deref(),
            query.from_year,
            query.to_year,
            offset,
            size,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let error: Subsonic = SubsonicError::error_generic().wrap(e.to_string()).into();
            let wrapper = JsonWrapper { subsonic_response: error };
            return HttpResponse::Ok().json(wrapper);
        }
    };

    let album_children: Vec<Child> = albums.into_iter().map(|album| Child::from(album)).collect();

    let response: Subsonic = AlbumList {
        album: album_children,
    }
    .into();

    let wrapper = JsonWrapper { subsonic_response: response };
    HttpResponse::Ok()
        .insert_header(("x-total-count", count.to_string()))
        .json(wrapper)
}

pub async fn get_album_list2(
    state: web::Data<AppState>,
    query: web::Query<GetAlbumListQuery>,
) -> impl Responder {
    let album_dao = AlbumDaoImpl::new(state.db.clone());
    let usecase = GetAlbumList::new(Arc::new(album_dao));

    let offset = query.offset.unwrap_or(0);
    let size = query.size.unwrap_or(10);

    let (albums, count) = match usecase
        .handle(
            &query.r#type,
            query.genre.as_deref(),
            query.from_year,
            query.to_year,
            offset,
            size,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let error: Subsonic = SubsonicError::error_generic().wrap(e.to_string()).into();
            let wrapper = JsonWrapper { subsonic_response: error };
            return HttpResponse::Ok().json(wrapper);
        }
    };

    let album_id3_list: Vec<AlbumID3> = albums
        .into_iter()
        .map(|album| AlbumID3::new(album))
        .collect();

    let response: Subsonic = AlbumList2 {
        album: album_id3_list,
    }
    .into();

    let wrapper = JsonWrapper { subsonic_response: response };
    HttpResponse::Ok()
        .insert_header(("x-total-count", count.to_string()))
        .json(wrapper)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRandomSongsQuery {
    pub size: Option<i32>,
    pub genre: Option<String>,
    pub from_year: Option<i32>,
    pub to_year: Option<i32>,
}

pub async fn get_random_songs(
    state: web::Data<AppState>,
    query: web::Query<GetRandomSongsQuery>,
) -> impl Responder {
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());
    let usecase = GetRandomSongs::new(Arc::new(audio_file_dao));

    let size = query.size.unwrap_or(10).min(500);

    let songs = match usecase
        .handle(query.genre.as_deref(), query.from_year, query.to_year, size)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let error: Subsonic = SubsonicError::error_generic().wrap(e.to_string()).into();
            return error;
        }
    };

    let song_children: Vec<Child> = songs.into_iter().map(|song| Child::from(song)).collect();

    let response: Subsonic = RandomSongs(Songs {
        song: song_children,
    })
    .into();

    response
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSongsByGenreQuery {
    pub genre: String,
    pub count: Option<i32>,
    pub offset: Option<i32>,
}

pub async fn get_songs_by_genre(
    state: web::Data<AppState>,
    query: web::Query<GetSongsByGenreQuery>,
) -> impl Responder {
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());
    let usecase = GetSongsByGenre::new(Arc::new(audio_file_dao));

    let count = query.count.unwrap_or(10).min(500);
    let offset = query.offset.unwrap_or(0);

    let songs = match usecase.handle(&query.genre, offset, count).await {
        Ok(result) => result,
        Err(e) => {
            let error: Subsonic = SubsonicError::error_generic().wrap(e.to_string()).into();
            return error;
        }
    };

    let song_children: Vec<Child> = songs.into_iter().map(|song| Child::from(song)).collect();

    let response: Subsonic = SongsByGenre(Songs {
        song: song_children,
    })
    .into();

    response
}

pub async fn get_starred(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    // 从 request extensions 中获取用户
    let user = match req.extensions().get::<domain::user::User>() {
        Some(user) => user.clone(),
        None => {
            let error: Subsonic = SubsonicError::error_generic().wrap("User not found".to_string()).into();
            return error;
        }
    };
    
    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let album_dao = AlbumDaoImpl::new(state.db.clone());
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());
    let token_service = Arc::new(JwtTokenService::new(
        &state.app_cfg.jwt_secret(),
        state.app_cfg.jwt_expire_secs(),
    ));
    
    let usecase = GetStarred::with_token_service(
        Arc::new(artist_dao),
        Arc::new(album_dao),
        Arc::new(audio_file_dao),
        token_service,
    );

    let (artists_with_tokens, albums, audio_files) = match usecase.handle_with_tokens(user.id.as_i64()).await {
        Ok(result) => result,
        Err(e) => {
            let error: Subsonic = SubsonicError::error_generic().wrap(e.to_string()).into();
            return error;
        }
    };

    let base_url = state.app_cfg.base_url();

    // 转换艺术家为响应对象（token 已在 application 层生成）
    let artist_list: Vec<Artist> = artists_with_tokens
        .into_iter()
        .map(|artist_with_token| {
            let artist_image_url = crate::subsonic::helper::image_url(
                &base_url,
                &artist_with_token.cover_art_token,
                300,
            );
            Artist::new_from_dto(artist_with_token, artist_image_url)
        })
        .collect();

    // 转换专辑为 Child
    let album_list: Vec<Child> = albums.into_iter().map(|album| Child::from(album)).collect();

    // 转换音频文件为 Child
    let song_list: Vec<Child> = audio_files
        .into_iter()
        .map(|song| Child::from(song))
        .collect();

    let response: Subsonic = Starred {
        artist: if artist_list.is_empty() {
            None
        } else {
            Some(artist_list)
        },
        album: if album_list.is_empty() {
            None
        } else {
            Some(album_list)
        },
        song: if song_list.is_empty() {
            None
        } else {
            Some(song_list)
        },
    }
    .into();

    response
}

pub async fn get_starred2(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    // 从 request extensions 中获取用户
    let user = match req.extensions().get::<domain::user::User>() {
        Some(user) => user.clone(),
        None => {
            let error: Subsonic = SubsonicError::error_generic().wrap("User not found".to_string()).into();
            return error;
        }
    };
    
    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let album_dao = AlbumDaoImpl::new(state.db.clone());
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());
    let token_service = Arc::new(JwtTokenService::new(
        &state.app_cfg.jwt_secret(),
        state.app_cfg.jwt_expire_secs(),
    ));
    
    let usecase = GetStarred::with_token_service(
        Arc::new(artist_dao),
        Arc::new(album_dao),
        Arc::new(audio_file_dao),
        token_service,
    );

    let (artists_with_tokens, albums, audio_files) = match usecase.handle_with_tokens(user.id.as_i64()).await {
        Ok(result) => result,
        Err(e) => {
            let error: Subsonic = SubsonicError::error_generic().wrap(e.to_string()).into();
            return error;
        }
    };

    let base_url = state.app_cfg.base_url();

    // 转换艺术家为 ArtistID3（token 已在 application 层生成）
    let artist_id3_list: Vec<ArtistID3> = artists_with_tokens
        .into_iter()
        .map(|artist_with_token| {
            let artist_image_url = crate::subsonic::helper::image_url(
                &base_url,
                &artist_with_token.cover_art_token,
                300,
            );
            ArtistID3::new(
                artist_with_token.artist,
                artist_with_token.cover_art_token,
                artist_image_url,
            )
        })
        .collect();

    // 转换专辑为 AlbumID3
    let album_id3_list: Vec<AlbumID3> = albums
        .into_iter()
        .map(|album| AlbumID3::new(album))
        .collect();

    // 转换音频文件为 Child
    let song_list: Vec<Child> = audio_files
        .into_iter()
        .map(|song| Child::from(song))
        .collect();

    let response: Subsonic = Starred2 {
        artist: artist_id3_list,
        album: album_id3_list,
        song: song_list,
    }
    .into();

    response
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetArtistListQuery {
    /// 列表类型: frequent/mostPlayed, recent/recentlyPlayed
    pub r#type: String,
    pub size: Option<i32>,
}

pub async fn get_artist_list(
    state: web::Data<AppState>,
    query: web::Query<GetArtistListQuery>,
) -> impl Responder {
    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let usecase = GetArtistList::new(Arc::new(artist_dao));

    let size = query.size.unwrap_or(10).min(500);

    let artists = match usecase.handle(&query.r#type, size).await {
        Ok(result) => result,
        Err(e) => {
            let error: Subsonic = SubsonicError::error_generic().wrap(e.to_string()).into();
            return error;
        }
    };

    let base_url = state.app_cfg.base_url();
    let token_service = JwtTokenService::new(
        &state.app_cfg.jwt_secret(),
        state.app_cfg.jwt_expire_secs(),
    );

    let artist_id3_list: Vec<ArtistID3> = artists
        .into_iter()
        .map(|artist| {
            let cover_art_id = format!("ar-{}", artist.id);
            let cover_art_token = token_service
                .issue_sub(&cover_art_id)
                .unwrap_or_default();
            let artist_image_url = crate::subsonic::helper::image_url(
                &base_url,
                &cover_art_token,
                300,
            );
            ArtistID3::new(artist, cover_art_token, artist_image_url)
        })
        .collect();

    let response: Subsonic = ArtistList::new(artist_id3_list).into();
    response
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSongsListQuery {
    /// 列表类型: frequent/mostPlayed, recent/recentlyPlayed
    pub r#type: String,
    pub size: Option<i32>,
}

pub async fn get_songs_list(
    state: web::Data<AppState>,
    query: web::Query<GetSongsListQuery>,
) -> impl Responder {
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());
    let usecase = GetSongsList::new(Arc::new(audio_file_dao));

    let size = query.size.unwrap_or(10).min(500);

    let songs = match usecase.handle(&query.r#type, size).await {
        Ok(result) => result,
        Err(e) => {
            let error: Subsonic = SubsonicError::error_generic().wrap(e.to_string()).into();
            return error;
        }
    };

    let song_children: Vec<Child> = songs.into_iter().map(|song| Child::from(song)).collect();

    let response: Subsonic = SongList { song: song_children }.into();
    response
}
