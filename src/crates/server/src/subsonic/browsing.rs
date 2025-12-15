use crate::consts;
use crate::subsonic::response::error::SubsonicError;
use crate::subsonic::response::{artist::Indexes, music_folder::MusicFolders, Subsonic};
use crate::AppState;
use actix_web::{
    error::ResponseError, http::StatusCode, middleware::from_fn, web, web::Json, web::Path,
    HttpResponse, Responder, Scope,
};
use application::query::artist::ArtistIndexRule;
use application::query::artist::ArtistService;
use application::query::dao::{AlbumDao, ArtistDao, AudioFileDao};
use application::query::get_album::GetAlbum;
use application::query::get_album_info::GetAlbumInfo;
use application::query::get_artist::GetArtist;
use application::query::get_artist_info::GetArtistInfo;
use application::query::get_genres::GetGenres;
use application::query::get_music_folders::GetMusicFolders;
use application::query::get_similar_songs::GetSimilarSongs;
use application::query::get_song::GetSong;
use application::query::get_top_songs::GetTopSongs;
use infra::auth::{AuthConfig, JwtTokenService};
use infra::repository::postgres::query::album::AlbumDaoImpl;
use infra::repository::postgres::query::artist::ArtistDaoImpl;
use infra::repository::postgres::query::audio_file::AudioFileDaoImpl;
use infra::repository::postgres::query::music_folder::MusicFolderDaoImpl;
use serde::Deserialize;
use std::sync::Arc;

pub async fn get_music_folders(state: web::Data<AppState>) -> Subsonic {
    let media_folder_dao = MusicFolderDaoImpl::new(state.db.clone());
    let usecase = GetMusicFolders::new(Arc::new(media_folder_dao));
    match usecase.handle().await {
        Ok(folders) => MusicFolders::new(folders).into(),
        Err(e) => SubsonicError::error_generic().wrap(e.to_string()).into(),
    }
}

pub async fn get_indexes(state: web::Data<AppState>) -> Subsonic {
    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let index_groups = state.app_cfg.indexgroups();
    let index_rule = ArtistIndexRule::new(&index_groups, true);
    let token_service = Arc::new(JwtTokenService::new(
        &state.app_cfg.jwt_secret(),
        state.app_cfg.jwt_expire_secs(),
    ));
    let usecase =
        ArtistService::with_token_service(Arc::new(artist_dao), index_rule, token_service);
    let ignored_articles = state.app_cfg.ignored_articles().join(" ");
    let base_url = state.app_cfg.base_url();
    match usecase.get_indexes_with_tokens().await {
        Ok(indexes) => Indexes::new(ignored_articles, indexes, &base_url).into(),
        Err(e) => SubsonicError::error_generic().wrap(e.to_string()).into(),
    }
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetArtistsQuery {
    /// 可选的音乐文件夹 ID，不提供时返回所有艺术家
    pub music_folder_id: Option<i64>,
}

use log::info;
pub async fn get_artists(
    state: web::Data<AppState>,
    query: web::Query<GetArtistsQuery>,
) -> Subsonic {
    use crate::subsonic::response::artist::Artists;
    use application::query::dao::MusicFolderDao;
    use infra::repository::postgres::query::music_folder::MusicFolderDaoImpl;

    // 获取 last_scan_at：如果指定了 music_folder_id 则从该文件夹获取，否则获取最新的
    let last_scan_at = if let Some(folder_id) = query.music_folder_id {
        let music_folder_dao = MusicFolderDaoImpl::new(state.db.clone());
        match music_folder_dao.get_by_id(folder_id).await {
            Ok(Some(folder)) => folder.last_scan_at,
            Ok(None) => {
                return SubsonicError::error_generic()
                    .wrap("Music folder not found".to_string())
                    .into();
            }
            Err(e) => return SubsonicError::error_generic().wrap(e.to_string()).into(),
        }
    } else {
        // 没有指定文件夹时，获取所有文件夹中最新的 last_scan_at
        let music_folder_dao = MusicFolderDaoImpl::new(state.db.clone());
        match music_folder_dao.get_all().await {
            Ok(folders) => folders
                .into_iter()
                .map(|f| f.last_scan_at)
                .max()
                .unwrap_or_else(|| chrono::Utc::now().naive_utc()),
            Err(_) => chrono::Utc::now().naive_utc(),
        }
    };

    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let index_groups = state.app_cfg.indexgroups();
    let index_rule = ArtistIndexRule::new(&index_groups, true);
    let token_service = Arc::new(JwtTokenService::new(
        &state.app_cfg.jwt_secret(),
        state.app_cfg.jwt_expire_secs(),
    ));
    let usecase =
        ArtistService::with_token_service(Arc::new(artist_dao), index_rule, token_service);
    let ignored_articles = state.app_cfg.ignored_articles().join(" ");
    let base_url = state.app_cfg.base_url();
    match usecase.get_artists_with_tokens().await {
        Ok(artists) => Artists::new(artists, last_scan_at, ignored_articles, &base_url).into(),
        Err(e) => SubsonicError::error_generic().wrap(e.to_string()).into(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetArtistQuery {
    pub id: i64,
}

use crate::subsonic::response::album::AlbumID3;
use crate::subsonic::response::artist::{ArtistID3, ArtistWithAlbumsID3};
pub async fn get_artist(state: web::Data<AppState>, query: web::Query<GetArtistQuery>) -> Subsonic {
    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let album_dao = AlbumDaoImpl::new(state.db.clone());
    let token_service = Arc::new(JwtTokenService::new(
        &state.app_cfg.jwt_secret(),
        state.app_cfg.jwt_expire_secs(),
    ));
    let usecase = GetArtist::new(Arc::new(artist_dao), Arc::new(album_dao), token_service);
    let artist_dto = match usecase.handle(query.id).await {
        Ok(dto) => dto,
        Err(e) => return SubsonicError::error_generic().wrap(e.to_string()).into(),
    };

    // 接口层负责 URL 生成（展示层关注点）
    let base_url = state.app_cfg.base_url();
    // 生成艺术家图片 URL（使用 artist_image_token）
    let artist_image_url = crate::subsonic::helper::image_url(
        &base_url,
        &artist_dto.cover_art_token,
        300, // 使用中等尺寸
    );

    let artist_id3 = ArtistID3::new(
        artist_dto.artist,
        artist_dto.cover_art_token,
        artist_image_url,
    );
    let albums_id3 = artist_dto
        .albums
        .into_iter()
        .map(|album| AlbumID3::new(album))
        .collect();
    ArtistWithAlbumsID3::new(artist_id3, albums_id3).into()
}

pub async fn get_genres(state: web::Data<AppState>) -> Subsonic {
    use crate::subsonic::response::genre::Genres;
    use infra::repository::postgres::query::genre::GenreDaoImpl;
    let genre_dao = GenreDaoImpl::new(state.db.clone());
    let usecase = GetGenres::new(Arc::new(genre_dao));
    match usecase.handle().await {
        Ok(genres) => Genres::new(genres).into(),
        Err(e) => SubsonicError::error_generic().wrap(e.to_string()).into(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAlbumQuery {
    pub id: i64,
}

use crate::subsonic::response::album::AlbumWithSongsID3;
use crate::subsonic::response::directory::Child;
pub async fn get_album(state: web::Data<AppState>, query: web::Query<GetAlbumQuery>) -> Subsonic {
    let album_dao = AlbumDaoImpl::new(state.db.clone());
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());
    let usecase = GetAlbum::new(Arc::new(album_dao), Arc::new(audio_file_dao));
    let (album, audio_files) = match usecase.handle(query.id).await {
        Ok(result) => result,
        Err(e) => return SubsonicError::error_generic().wrap(e.to_string()).into(),
    };

    let songs: Vec<Child> = audio_files
        .into_iter()
        .map(|audio_file| Child::from(audio_file))
        .collect();

    let album_id3 = AlbumID3::new(album);
    AlbumWithSongsID3 {
        album: album_id3,
        song: songs,
    }
    .into()
}

use crate::subsonic::response::song::Song;
#[derive(Deserialize)]
pub struct GetSongQuery {
    pub id: i64,
}

pub async fn get_song(state: web::Data<AppState>, query: web::Query<GetSongQuery>) -> Subsonic {
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());
    let usecase = GetSong::new(Arc::new(audio_file_dao));
    let audio_file = match usecase.handle(query.id).await {
        Ok(audio_file) => audio_file,
        Err(e) => return SubsonicError::error_generic().wrap(e.to_string()).into(),
    };

    Song {
        child: Child::from(audio_file),
    }
    .into()
}

use crate::subsonic::response::artist::ArtistInfo;
#[derive(Deserialize)]
pub struct GetArtistInfoQuery {
    pub id: i64,
}
pub async fn get_artist_info(
    state: web::Data<AppState>,
    query: web::Query<GetArtistInfoQuery>,
) -> Subsonic {
    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let token_service = Arc::new(JwtTokenService::new(
        &state.app_cfg.jwt_secret(),
        state.app_cfg.jwt_expire_secs(),
    ));
    let usecase = GetArtistInfo::new(Arc::new(artist_dao), token_service);
    let artist_info_dto = match usecase.handle(query.id).await {
        Ok(dto) => dto,
        Err(e) => return SubsonicError::error_generic().wrap(e.to_string()).into(),
    };

    // 接口层负责 URL 生成（展示层关注点）
    let base_url = state.app_cfg.base_url();
    let image_urls =
        crate::subsonic::helper::generate_image_urls(&base_url, &artist_info_dto.cover_art_token);

    // 创建响应对象
    ArtistInfo::new(
        artist_info_dto.artist_info,
        image_urls.small,
        image_urls.medium,
        image_urls.large,
    )
    .into()
}

use crate::subsonic::response::album::AlbumInfo;
#[derive(Deserialize)]
pub struct GetAlbumInfoQuery {
    pub id: i64,
}
pub async fn get_album_info(
    state: web::Data<AppState>,
    query: web::Query<GetAlbumInfoQuery>,
) -> Subsonic {
    let album_dao = AlbumDaoImpl::new(state.db.clone());
    let token_service = Arc::new(JwtTokenService::new(
        &state.app_cfg.jwt_secret(),
        state.app_cfg.jwt_expire_secs(),
    ));
    let usecase = GetAlbumInfo::new(Arc::new(album_dao), token_service);
    let album_info_dto = match usecase.handle(query.id).await {
        Ok(dto) => dto,
        Err(e) => return SubsonicError::error_generic().wrap(e.to_string()).into(),
    };

    // 接口层负责 URL 生成（展示层关注点）
    let base_url = state.app_cfg.base_url();
    let image_urls =
        crate::subsonic::helper::generate_image_urls(&base_url, &album_info_dto.cover_art_token);

    // 创建响应对象
    AlbumInfo::new(
        album_info_dto.album_info,
        image_urls.small,
        image_urls.medium,
        image_urls.large,
    )
    .into()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTopSongsQuery {
    pub artist: String,
    pub count: Option<i32>,
}

use crate::subsonic::response::song::TopSongs;
pub async fn get_top_songs(
    state: web::Data<AppState>,
    query: web::Query<GetTopSongsQuery>,
) -> Subsonic {
    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());
    let usecase = GetTopSongs::new(Arc::new(artist_dao), Arc::new(audio_file_dao));

    // query the top songs by artist (按播放次数排序，限制数量)
    let songs = match usecase
        .handle(&query.artist, query.count.unwrap_or(50))
        .await
    {
        Ok(songs) => songs,
        Err(e) => return SubsonicError::error_generic().wrap(e.to_string()).into(),
    };

    // 将 AudioFile 转换为 Child
    let children: Vec<Child> = songs.into_iter().map(Child::from).collect();
    TopSongs { song: children }.into()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSimilarSongsQuery {
    pub id: String,
    pub count: Option<i32>,
}

use crate::subsonic::response::song::SimilarSongs;
pub async fn get_similar_songs(
    state: web::Data<AppState>,
    query: web::Query<GetSimilarSongsQuery>,
) -> Subsonic {
    // 解析 artist_id（从字符串转换为 i64）
    let artist_id: i64 = match query.id.parse() {
        Ok(id) => id,
        Err(_) => {
            return SubsonicError::error_generic()
                .wrap(format!("Invalid artist ID: {}", query.id))
                .into();
        }
    };

    let artist_dao = ArtistDaoImpl::new(state.db.clone());
    let audio_file_dao = AudioFileDaoImpl::new(state.db.clone());
    let usecase = GetSimilarSongs::new(Arc::new(artist_dao), Arc::new(audio_file_dao));

    // query the similar artist' songs
    let songs = match usecase.handle(artist_id, query.count.unwrap_or(50)).await {
        Ok(songs) => songs,
        Err(e) => return SubsonicError::error_generic().wrap(e.to_string()).into(),
    };

    // 将 AudioFile 转换为 Child
    let children: Vec<Child> = songs.into_iter().map(Child::from).collect();
    SimilarSongs { song: children }.into()
}
