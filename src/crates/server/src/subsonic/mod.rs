pub mod bookmarks;
pub mod browsing;
pub mod helper;
pub mod media_annotation;
pub mod media_retrieval;
pub mod playlists;
pub mod response;
pub mod scan;
pub mod searching;
pub mod song_album_list;
pub mod system;
pub mod users;
use crate::consts;
use crate::middleware::other;
use actix_web::{middleware::from_fn, web};

pub fn configure_service(svc: &mut web::ServiceConfig) {
    // 中间件执行顺序：从下到上包装，从上到下执行
    // 1. check_required_parameters - 验证必需参数 (u, v, c)
    // 2. subsonic_authenticator - 用户认证
    svc.service(
        web::scope(consts::URL_PATH_SUBSONIC_API)
            .configure(configure_routes)
            .wrap(from_fn(move |req, next| {
                other::subsonic_authenticator(req, next)
            }))
            .wrap(from_fn(move |req, next| {
                other::check_required_parameters(req, next)
            })),
    );
}

/// 注册所有 Subsonic API 路由（包括 .view 后缀兼容）
fn configure_routes(cfg: &mut web::ServiceConfig) {
    // System
    register_get("ping", system::ping, cfg);

    // Browsing
    register_get("getMusicFolders", browsing::get_music_folders, cfg);
    register_get("getIndexes", browsing::get_indexes, cfg);
    register_get("getArtists", browsing::get_artists, cfg);
    register_get("getArtist", browsing::get_artist, cfg);
    register_get("getAlbum", browsing::get_album, cfg);
    register_get("getSong", browsing::get_song, cfg);
    register_get("getArtistInfo", browsing::get_artist_info, cfg);
    register_get("getArtistInfo2", browsing::get_artist_info, cfg);
    register_get("getAlbumInfo", browsing::get_album_info, cfg);
    register_get("getAlbumInfo2", browsing::get_album_info, cfg);
    register_get("getTopSongs", browsing::get_top_songs, cfg);
    register_get("getSimilarSongs", browsing::get_similar_songs, cfg);
    register_get("getSimilarSongs2", browsing::get_similar_songs, cfg);
    register_get("getGenres", browsing::get_genres, cfg);

    // Album/Song Lists
    register_get("getAlbumList", song_album_list::get_album_list, cfg);
    register_get("getAlbumList2", song_album_list::get_album_list2, cfg);
    register_get("getRandomSongs", song_album_list::get_random_songs, cfg);
    register_get("getSongsByGenre", song_album_list::get_songs_by_genre, cfg);
    register_get("getStarred", song_album_list::get_starred, cfg);
    register_get("getStarred2", song_album_list::get_starred2, cfg);
    register_get("getArtistList", song_album_list::get_artist_list, cfg);
    register_get("getSongsList", song_album_list::get_songs_list, cfg);

    // Media Annotation
    register_post("star", media_annotation::star, cfg);
    register_post("unstar", media_annotation::unstar, cfg);
    register_post("setRating", media_annotation::set_rating, cfg);
    register_post("scrobble", media_annotation::scrobble, cfg);

    // Playlists
    register_get("getPlaylists", playlists::get_playlists, cfg);
    register_get("getPlaylist", playlists::get_playlist, cfg);
    register_get_post("createPlaylist", playlists::create_playlist, cfg);
    register_get_post("updatePlaylist", playlists::update_playlist, cfg);
    register_get_post("deletePlaylist", playlists::delete_playlist, cfg);

    // Bookmarks
    register_get_post("savePlayQueue", bookmarks::save_play_queue, cfg);
    register_get("getPlayQueue", bookmarks::get_play_queue, cfg);

    // User Management
    register_get_post("createUser", users::create_user, cfg);
    register_get_post("updateUser", users::update_user, cfg);
    register_get_post("deleteUser", users::delete_user, cfg);
    register_get_post("changePassword", users::change_password, cfg);

    // Searching
    register_get_post("search", searching::search, cfg);
    register_get_post("search2", searching::search2, cfg);
    register_get_post("search3", searching::search3, cfg);

    // Media Retrieval
    register_get("getCoverArt", media_retrieval::get_cover_art, cfg);
    register_get("stream", media_retrieval::stream, cfg);

    // Scanning (OpenSubsonic standard - no library id parameter)
    register_get_post("startScan", scan::start_library_scan, cfg);
    register_get("getScanStatus", scan::get_scan_status, cfg);
}

/// 注册 GET 路由（同时注册 /name 和 /name.view）
fn register_get<F, Args>(name: &str, handler: F, cfg: &mut web::ServiceConfig)
where
    F: actix_web::Handler<Args> + Copy,
    Args: actix_web::FromRequest + 'static,
    F::Output: actix_web::Responder + 'static,
{
    cfg.service(web::resource(format!("/{}", name)).route(web::get().to(handler)));
    cfg.service(web::resource(format!("/{}.view", name)).route(web::get().to(handler)));
}

/// 注册 POST 路由（同时注册 /name 和 /name.view）
fn register_post<F, Args>(name: &str, handler: F, cfg: &mut web::ServiceConfig)
where
    F: actix_web::Handler<Args> + Copy,
    Args: actix_web::FromRequest + 'static,
    F::Output: actix_web::Responder + 'static,
{
    cfg.service(web::resource(format!("/{}", name)).route(web::post().to(handler)));
    cfg.service(web::resource(format!("/{}.view", name)).route(web::post().to(handler)));
}

/// 注册 GET+POST 路由（同时注册 /name 和 /name.view）
fn register_get_post<F, Args>(name: &str, handler: F, cfg: &mut web::ServiceConfig)
where
    F: actix_web::Handler<Args> + Copy,
    Args: actix_web::FromRequest + 'static,
    F::Output: actix_web::Responder + 'static,
{
    cfg.service(
        web::resource(format!("/{}", name))
            .route(web::get().to(handler))
            .route(web::post().to(handler)),
    );
    cfg.service(
        web::resource(format!("/{}.view", name))
            .route(web::get().to(handler))
            .route(web::post().to(handler)),
    );
}
