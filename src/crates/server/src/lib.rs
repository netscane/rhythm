pub mod auth;
pub mod consts;
pub mod middleware;
pub mod native_api;
pub mod resources;
pub mod subsonic;

use application::auth::AuthService;
use application::command::album::AlbumService;
use application::command::artist::ArtistService;
use application::command::audio_file::AudioFileService;
use application::command::cover_art::CoverArtService;
use application::command::genre::GenreService;
use application::command::media_parse::MediaFileParseService;
use application::command::shared::IdGenerator;
use application::event::coordinator::register::register_coordinators;
use application::event::event_bus::EventBus;
use application::event::handler::album::registry::register_handlers as register_album_handlers;
use application::event::handler::artist::registry::register_handlers as register_artist_handlers;
use application::event::handler::audio_file::registry::register_handlers as register_audio_file_handlers;
use application::event::handler::cover_art::registry::register_handlers as register_cover_art_handlers;
use application::event::handler::genre::registry::register_handlers as register_genre_handlers;
use application::event::handler::on_library_file_added::OnLibraryFileAddedHandler;
use application::event::handler::projector::registry::register_handlers;
use application::shared::SystemConfigStore;
use domain::library::LibraryEvent;
use infra::auth::{BcryptPasswordHasher, JwtTokenService};
use infra::config::AppConfigImpl;
use infra::event_bus::in_memory::InMemoryEventBus;
use infra::id_generator::SnowflakeIdGenerator;
use infra::Aes256GcmEncryptor;
use infra::metadata::audio_metadata_reader::AudioMetadataReaderImpl;
use infra::repository::postgres::command::system_config::SystemConfigStoreImpl;
use infra::repository::postgres::command::user::UserRepositoryImpl;

use infra::repository::buffered::command::{
    album::BufferedAlbumRepository, artist::BufferedArtistRepository,
    audio_file::BufferedAudioFileRepository, cover_art::BufferedCoverArtRepository,
    genre::BufferedGenreRepository,
};
use infra::repository::buffered::query::{
    album_stats::BufferedAlbumStatsRepository, genre_stats::BufferedGenreStatsRepository,
    participant_stats::BufferedParticipantStatsRepository,
};
use infra::repository::in_memory::scan_status::InMemoryScanStatusRepository;
use infra::repository::postgres::command::{
    album::AlbumRepositoryImpl, artist::ArtistRepositoryImpl, audio_file::AudioFileRepositoryImpl,
    cover_art::CoverArtRepositoryImpl, genre::GenreRepositoryImpl,
};
use infra::repository::postgres::query::genre::GenreStatsRepositoryImpl;
use infra::repository::postgres::query::{
    album_location::MysqlAlbumLocationRepository, album_stats::MysqlAlbumStatsRepository,
    artist_location::MysqlArtistLocationRepository,
    participant_stats::MysqlParticipantStatsRepository,
    playback_history::PlaybackHistoryRepositoryImpl,
};
use infra::storage::factory::StorageClientFactoryImpl;
use infra::{CoverArtCacheImpl, FfmpegStreamer, StreamCacheImpl};
use model::scan_status::ScanStatusRepository;
use sea_orm::DatabaseConnection;
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DbBackend, Statement};
use std::sync::Arc;
use tokio::time::Duration;

pub struct AppState {
    pub app_cfg: AppConfigImpl,
    pub db: DatabaseConnection,
    pub id_generator: Arc<dyn IdGenerator>,
    pub event_bus: InMemoryEventBus,
    pub scan_repo: Arc<dyn ScanStatusRepository + Send + Sync>,
    pub cover_art_cache: Arc<CoverArtCacheImpl>,
    pub stream_cache: Arc<StreamCacheImpl>,
    pub transcoder: Arc<FfmpegStreamer>,
}

impl AppState {
    pub async fn init_db(db_url: &str) -> DatabaseConnection {
        use log::info;
        use std::time::Duration;

        let mut opt = ConnectOptions::new(db_url.to_string());
        opt.max_connections(90)
            .min_connections(20)
            .connect_timeout(Duration::from_secs(3))
            .acquire_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(60))
            .max_lifetime(Duration::from_secs(300))
            .sqlx_logging(false)
            .sqlx_logging_level(log::LevelFilter::Info);

        let db = Database::connect(opt)
            .await
            .expect("Failed to connect to database");

        let backend = DbBackend::Postgres;
        db.execute(Statement::from_string(backend, "SELECT 1".to_owned()))
            .await
            .expect("Failed to execute test query");

        info!("Database connection pool initialized successfully");
        db
    }

    pub async fn new(db: DatabaseConnection, app_cfg: AppConfigImpl) -> Self {
        let id_generator: Arc<dyn IdGenerator> = Arc::new(SnowflakeIdGenerator::new(1).unwrap());
        let event_bus = InMemoryEventBus::new();

        // 初始化封面缓存
        let cache_cfg = app_cfg.cache();
        let cover_art_cache = Arc::new(
            CoverArtCacheImpl::new(cache_cfg.cover_art_cache_path(), cache_cfg.ttl_secs)
                .expect("Failed to create cover art cache"),
        );

        // 初始化转码配置和缓存
        let transcoding_cfg = app_cfg.transcoding();
        let stream_cache = Arc::new(
            StreamCacheImpl::new(
                transcoding_cfg.cache_path(&cache_cfg.data_dir),
                transcoding_cfg.cache_ttl_secs,
            )
            .expect("Failed to create stream cache"),
        );
        let transcoder = Arc::new(FfmpegStreamer::new(
            transcoding_cfg.ffmpeg_path.clone(),
            transcoding_cfg.chunk_size,
        ));

        Self {
            app_cfg,
            db,
            id_generator,
            event_bus,
            scan_repo: Arc::new(InMemoryScanStatusRepository::new()),
            cover_art_cache,
            stream_cache,
            transcoder,
        }
    }
}

pub async fn init_admin_user(state: &AppState) {
    use log::{info, warn};
    use rand::Rng;

    let config_store = SystemConfigStoreImpl::new(state.db.clone());

    // Check if first_time key exists
    match config_store.get_string("first_time").await {
        Ok(Some(v)) => {
            // Already initialized, skip
            info!(
                "System already initialized (first_time={}), skipping admin creation",
                v
            );
            return;
        }
        Ok(None) => {
            // First time, create admin user
            info!("First time setup, will create admin user");
        }
        Err(e) => {
            warn!("Failed to check first_time config: {}", e);
            return;
        }
    }

    // Generate random password (12 characters)
    let password: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();

    let user_repo: Arc<dyn domain::user::UserRepository> =
        Arc::new(UserRepositoryImpl::new(state.db.clone()));
    let hasher: Arc<dyn application::auth::PasswordHasher> =
        Arc::new(BcryptPasswordHasher::new(10));
    let encryptor: Arc<dyn application::auth::PasswordEncryptor> = Arc::new(
        Aes256GcmEncryptor::new(&state.app_cfg.password_encryption_key())
            .expect("Failed to create password encryptor"),
    );
    let token_svc: Arc<dyn application::auth::TokenService> =
        Arc::new(JwtTokenService::new("temp_secret", 3600));

    let auth_service = AuthService::new(user_repo, hasher, encryptor, token_svc, state.id_generator.clone());

    match auth_service.create_admin("admin", &password).await {
        Ok(()) => {
            info!("===========================================");
            info!("  Admin user created successfully!");
            info!("  Username: admin");
            info!("  Password: {}", password);
            info!("  Please change the password after login!");
            info!("===========================================");

            // Set first_time flag
            if let Err(e) = config_store.set_string("first_time", "false").await {
                warn!("Failed to set first_time config: {}", e);
            }
        }
        Err(e) => {
            // User already exists or other error
            panic!("Admin user not created: {}", e);
        }
    }
}

pub async fn setup_event_bus(state: &mut AppState) {
    setup_application_handlers(state).await;

    // 创建共享的 buffered repositories
    let album_repository_impl =
        AlbumRepositoryImpl::new(state.db.clone(), state.id_generator.clone());
    let album_repository: Arc<dyn domain::album::AlbumRepository> = BufferedAlbumRepository::new(
        album_repository_impl,
        100,                    // cache_capacity: 缓存容量
        3,                      // concurrency: 并发数
        Duration::from_secs(5), // flush_timeout: 超时时间（即使未达到容量也 flush）
    );

    let artist_repository_impl =
        ArtistRepositoryImpl::new(state.db.clone(), state.id_generator.clone());
    let artist_repository: Arc<dyn domain::artist::ArtistRepository> =
        BufferedArtistRepository::new(
            artist_repository_impl,
            100,                    // cache_capacity: 缓存容量
            5,                      // concurrency: 并发数
            Duration::from_secs(5), // flush_timeout: 超时时间（即使未达到容量也 flush）
        );

    let genre_repository_impl = GenreRepositoryImpl::new(state.db.clone());
    let genre_repository: Arc<dyn domain::genre::GenreRepository> = BufferedGenreRepository::new(
        genre_repository_impl,
        50,                     // cache_capacity: 缓存容量
        3,                      // concurrency: 并发数
        Duration::from_secs(5), // flush_timeout: 超时时间（即使未达到容量也 flush）
    );

    let audio_file_repository = AudioFileRepositoryImpl::new(state.db.clone());
    /*
    let audio_file_repository: Arc<dyn domain::audio_file::AudioFileRepository> =
        Arc::new(audio_file_repository);
        */
    let audio_file_repository: Arc<dyn domain::audio_file::AudioFileRepository> =
        BufferedAudioFileRepository::new(
            audio_file_repository,
            1000,                   // cache_capacity: 缓存容量
            10,                     // concurrency: 并发数
            Duration::from_secs(5), // flush_timeout: 超时时间（即使未达到容量也 flush）
        );

    // 创建共享的其他 repositories
    let cover_art_repository_impl =
        CoverArtRepositoryImpl::new(state.db.clone(), state.id_generator.clone());
    let cover_art_repository: Arc<dyn domain::cover_art::CoverArtRepository> =
        BufferedCoverArtRepository::new(
            cover_art_repository_impl,
            100,                    // cache_capacity: 缓存容量
            3,                      // concurrency: 并发数
            Duration::from_secs(5), // flush_timeout: 超时时间（即使未达到容量也 flush）
        );

    setup_domain_handlers(
        state,
        album_repository.clone(),
        artist_repository.clone(),
        genre_repository.clone(),
        audio_file_repository.clone(),
        cover_art_repository.clone(),
    )
    .await;
    setup_projector_handlers(state).await;
    setup_coordinators(
        state,
        album_repository,
        artist_repository,
        audio_file_repository,
        cover_art_repository,
    )
    .await;
}

pub async fn setup_application_handlers(state: &mut AppState) {
    let media_file_parse_service = MediaFileParseService::new(
        Arc::new(state.event_bus.clone()),
        Arc::new(StorageClientFactoryImpl::new()),
        Arc::new(AudioMetadataReaderImpl::new()),
    );
    let on_library_file_added_handler = OnLibraryFileAddedHandler::new(media_file_parse_service);
    state
        .event_bus
        .subscribe::<LibraryEvent>(Arc::new(on_library_file_added_handler))
        .await;
}

async fn setup_coordinators(
    state: &mut AppState,
    album_repository: Arc<dyn domain::album::AlbumRepository>,
    artist_repository: Arc<dyn domain::artist::ArtistRepository>,
    audio_file_repository: Arc<dyn domain::audio_file::AudioFileRepository>,
    cover_art_repository: Arc<dyn domain::cover_art::CoverArtRepository>,
) {
    let ignored_articles = state.app_cfg.ignored_articles();
    let artist_name_normalizer = Arc::new(infra::normalize::ArtistNameNormalizerImpl::new(
        &ignored_articles,
    ));
    let album_name_normalizer = Arc::new(infra::normalize::AlbumNameNormalizerImpl::new(
        &ignored_articles,
    ));

    register_coordinators(
        &mut state.event_bus,
        album_repository,
        artist_repository,
        audio_file_repository,
        cover_art_repository,
        state.id_generator.clone(),
        artist_name_normalizer,
        album_name_normalizer,
    )
    .await;
}

async fn setup_domain_handlers(
    state: &mut AppState,
    album_repository: Arc<dyn domain::album::AlbumRepository>,
    artist_repository: Arc<dyn domain::artist::ArtistRepository>,
    genre_repository: Arc<dyn domain::genre::GenreRepository>,
    audio_file_repository: Arc<dyn domain::audio_file::AudioFileRepository>,
    cover_art_repository: Arc<dyn domain::cover_art::CoverArtRepository>,
) {
    let ignored_articles = state.app_cfg.ignored_articles();
    let artist_name_normalizer = Arc::new(infra::normalize::ArtistNameNormalizerImpl::new(
        &ignored_articles,
    ));
    let album_name_normalizer = Arc::new(infra::normalize::AlbumNameNormalizerImpl::new(
        &ignored_articles,
    ));

    // Create services
    let audio_file_service = AudioFileService::new(
        state.id_generator.clone(),
        audio_file_repository.clone(),
        Arc::new(state.event_bus.clone()),
    );

    let album_service = AlbumService::new(
        state.id_generator.clone(),
        album_repository.clone(),
        album_name_normalizer,
        Arc::new(state.event_bus.clone()),
    );

    let artist_service = ArtistService::new(
        state.id_generator.clone(),
        artist_repository.clone(),
        artist_name_normalizer,
        Arc::new(state.event_bus.clone()),
    );

    let cover_art_service = CoverArtService::new(
        cover_art_repository.clone(),
        state.id_generator.clone(),
        Arc::new(state.event_bus.clone()),
    );

    let genre_service = GenreService::new(
        state.id_generator.clone(),
        genre_repository.clone(),
        Arc::new(state.event_bus.clone()),
    );

    // Register handlers
    register_audio_file_handlers(&mut state.event_bus, audio_file_service).await;
    register_album_handlers(&mut state.event_bus, album_service).await;
    register_artist_handlers(&mut state.event_bus, artist_service).await;
    register_cover_art_handlers(&mut state.event_bus, cover_art_service).await;
    register_genre_handlers(&mut state.event_bus, genre_service).await;
}

async fn setup_projector_handlers(state: &mut AppState) {
    // Create all repositories
    let album_location_repository = Arc::new(MysqlAlbumLocationRepository::new(state.db.clone()));
    let album_stats_repository = MysqlAlbumStatsRepository::new(state.db.clone());
    let album_stats_repository = BufferedAlbumStatsRepository::new(
        album_stats_repository,
        1000,                    // cache_capacity
        Duration::from_secs(30), // flush_timeout
    );
    let artist_location_repository = Arc::new(MysqlArtistLocationRepository::new(state.db.clone()));
    let genre_stats_repository = GenreStatsRepositoryImpl::new(state.db.clone());
    let genre_stats_repository = BufferedGenreStatsRepository::new(
        genre_stats_repository,
        100,                     // cache_capacity
        Duration::from_secs(30), // flush_timeout
    );
    let participant_stats_repository = MysqlParticipantStatsRepository::new(state.db.clone());
    let participant_stats_repository = BufferedParticipantStatsRepository::new(
        participant_stats_repository,
        2000,                    // cache_capacity
        Duration::from_secs(30), // flush_timeout
    );
    let playback_history_repository =
        Arc::new(PlaybackHistoryRepositoryImpl::new(state.db.clone()));
    let scan_status_repository = state.scan_repo.clone();

    // Register all projector handlers using the centralized function
    register_handlers(
        &mut state.event_bus,
        album_location_repository,
        album_stats_repository,
        artist_location_repository,
        genre_stats_repository,
        participant_stats_repository,
        playback_history_repository,
        scan_status_repository,
        state.id_generator.clone(),
    )
    .await;
}
