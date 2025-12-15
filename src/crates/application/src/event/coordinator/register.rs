use super::bind_to_album::BindToAlbumCoordinator;
use super::bind_to_artist::BindToArtistCoordinator;
use super::bind_to_audio_file::BindToAudioFileCoordinator;
use super::bind_to_cover_art::BindToCoverArtCoordinator;
use crate::command::album::AlbumService;
use crate::command::artist::ArtistService;
use crate::command::audio_file::AudioFileService;
use crate::command::cover_art::CoverArtService;
use crate::command::shared::IdGenerator;
use crate::event::event_bus::EventBus;
use domain::album::AlbumRepository;
use domain::artist::ArtistRepository;
use domain::audio_file::AudioFileRepository;
use domain::cover_art::CoverArtRepository;
use std::sync::Arc;

pub async fn register_coordinators<B: EventBus + Clone + 'static>(
    bus: &mut B,
    // 仓储依赖
    album_repository: Arc<dyn AlbumRepository>,
    artist_repository: Arc<dyn ArtistRepository>,
    audio_file_repository: Arc<dyn AudioFileRepository>,
    cover_art_repository: Arc<dyn CoverArtRepository>,
    // 服务依赖
    id_generator: Arc<dyn IdGenerator>,
    // 标准化器依赖
    artist_name_normalizer: Arc<dyn crate::command::artist::ArtistNameNormalizer>,
    album_name_normalizer: Arc<dyn crate::command::album::AlbumNameNormalizer>,
) {
    // 创建服务
    let audio_file_service = AudioFileService::new(
        id_generator.clone(),
        audio_file_repository.clone(),
        Arc::new(bus.clone()),
    );

    let album_service = AlbumService::new(
        id_generator.clone(),
        album_repository.clone(),
        album_name_normalizer,
        Arc::new(bus.clone()),
    );

    let artist_service = ArtistService::new(
        id_generator.clone(),
        artist_repository.clone(),
        artist_name_normalizer,
        Arc::new(bus.clone()),
    );

    let cover_art_service = CoverArtService::new(
        cover_art_repository.clone(),
        id_generator.clone(),
        Arc::new(bus.clone()),
    );

    // 创建协调器
    let bind_to_audio_file_coordinator = BindToAudioFileCoordinator::new(audio_file_service);
    let bind_to_artist_coordinator = BindToArtistCoordinator::new(artist_service);
    let bind_to_album_coordinator = BindToAlbumCoordinator::new(album_service);
    let bind_to_cover_art_coordinator = BindToCoverArtCoordinator::new(cover_art_service);

    // 注册协调器到事件总线
    // BindToAudioFileCoordinator 监听的事件
    bus.subscribe::<domain::artist::ArtistEvent>(Arc::new(bind_to_audio_file_coordinator.clone()))
        .await;
    bus.subscribe::<domain::album::AlbumEvent>(Arc::new(bind_to_audio_file_coordinator.clone()))
        .await;
    bus.subscribe::<domain::genre::GenreEvent>(Arc::new(bind_to_audio_file_coordinator.clone()))
        .await;
    bus.subscribe::<domain::audio_file::AudioFileEvent>(Arc::new(
        bind_to_audio_file_coordinator.clone(),
    ))
    .await;
    bus.subscribe::<crate::event::events::AppEvent>(Arc::new(bind_to_audio_file_coordinator))
        .await;

    // BindToArtistCoordinator 监听的事件
    bus.subscribe::<domain::artist::ArtistEvent>(Arc::new(bind_to_artist_coordinator.clone()))
        .await;
    bus.subscribe::<domain::genre::GenreEvent>(Arc::new(bind_to_artist_coordinator.clone()))
        .await;
    bus.subscribe::<crate::event::events::AppEvent>(Arc::new(bind_to_artist_coordinator))
        .await;

    // BindToAlbumCoordinator 监听的事件
    bus.subscribe::<domain::artist::ArtistEvent>(Arc::new(bind_to_album_coordinator.clone()))
        .await;
    bus.subscribe::<domain::genre::GenreEvent>(Arc::new(bind_to_album_coordinator.clone()))
        .await;
    bus.subscribe::<domain::album::AlbumEvent>(Arc::new(bind_to_album_coordinator.clone()))
        .await;
    bus.subscribe::<crate::event::events::AppEvent>(Arc::new(bind_to_album_coordinator))
        .await;

    // BindToCoverArtCoordinator 监听的事件
    bus.subscribe::<domain::audio_file::AudioFileEvent>(Arc::new(
        bind_to_cover_art_coordinator.clone(),
    ))
    .await;
    bus.subscribe::<domain::cover_art::CoverArtEvent>(Arc::new(bind_to_cover_art_coordinator))
        .await;
}
