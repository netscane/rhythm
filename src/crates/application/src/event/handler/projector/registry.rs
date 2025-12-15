use super::album_location::AlbumLocationHandler;
use super::album_stats::AlbumStatsHandler;
use super::artist_location::ArtistLocationHandler;
use super::genre_stats::GenreStatsHandler;
use super::participant_stats::ParticipantStatsHandler;
use super::playback_history::PlaybackHistoryEventHandler;
use super::scan_status::{ScanLifecycleEventHandler, ScanStatusEventHandler};
use crate::command::shared::IdGenerator;
use crate::event::event_bus::EventBus;
use crate::projector::album_location::AlbumLocationProjector;
use crate::projector::album_stats::AlbumStatsProjector;
use crate::projector::artist_location::ArtistLocationProjector;
use crate::projector::genre_stats::GenreStatsProjector;
use crate::projector::participant_stats::ParticipantStatsProjector;
use crate::projector::scan_status::ScanStatusProjectorImpl;
use model::album_location::AlbumLocationRepository;
use model::album_stats::AlbumStatsRepository;
use model::artist_location::ArtistLocationRepository;
use model::genre::GenreStatsRepository;
use model::participant_stats::ParticipantStatsRepository;
use model::playback_history::PlaybackHistoryRepository;
use model::scan_status::ScanStatusRepository;
use std::sync::Arc;

pub async fn register_handlers<B: EventBus + Clone + 'static>(
    bus: &mut B,
    // 仓储依赖
    album_location_repository: Arc<dyn AlbumLocationRepository>,
    album_stats_repository: Arc<dyn AlbumStatsRepository>,
    artist_location_repository: Arc<dyn ArtistLocationRepository>,
    genre_stats_repository: Arc<dyn GenreStatsRepository>,
    participant_stats_repository: Arc<dyn ParticipantStatsRepository>,
    playback_history_repository: Arc<dyn PlaybackHistoryRepository + Send + Sync>,
    scan_status_repository: Arc<dyn ScanStatusRepository + Send + Sync>,
    // 服务依赖
    id_generator: Arc<dyn IdGenerator>,
) {
    // 创建投影器
    let album_location_projector =
        AlbumLocationProjector::new(album_location_repository, id_generator.clone());
    let album_stats_projector = AlbumStatsProjector::new(album_stats_repository);

    let artist_location_projector =
        ArtistLocationProjector::new(artist_location_repository, id_generator.clone());
    let participant_stats_projector =
        ParticipantStatsProjector::new(participant_stats_repository.clone(), id_generator.clone());

    let genre_stats_projector_audio = GenreStatsProjector::new(genre_stats_repository.clone());
    let genre_stats_projector_album = GenreStatsProjector::new(genre_stats_repository);
    let scan_status_projector = Arc::new(ScanStatusProjectorImpl::new(scan_status_repository));

    // 创建处理器
    let album_location_handler = AlbumLocationHandler::new(album_location_projector);
    let album_stats_handler = AlbumStatsHandler::new(album_stats_projector);

    let artist_location_handler = ArtistLocationHandler::new(artist_location_projector);

    let participant_stats_handler =
        Arc::new(ParticipantStatsHandler::new(participant_stats_projector));

    let genre_stats_handler_audio = GenreStatsHandler::new(genre_stats_projector_audio);
    let genre_stats_handler_album = GenreStatsHandler::new(genre_stats_projector_album);
    let scan_status_handler = ScanStatusEventHandler::new(scan_status_projector.clone());
    let scan_lifecycle_handler = ScanLifecycleEventHandler::new(scan_status_projector);
    let playback_history_handler = PlaybackHistoryEventHandler::new(playback_history_repository);

    // 注册处理器到事件总线
    bus.subscribe::<domain::audio_file::AudioFileEvent>(Arc::new(album_location_handler))
        .await;
    bus.subscribe::<domain::audio_file::AudioFileEvent>(Arc::new(album_stats_handler))
        .await;

    /*
    bus.subscribe::<domain::audio_file::AudioFileEvent>(Arc::new(artist_location_handler))
        .await;
    */

    bus.subscribe::<domain::audio_file::AudioFileEvent>(participant_stats_handler.clone())
        .await;
    bus.subscribe::<domain::album::AlbumEvent>(participant_stats_handler.clone())
        .await;
    bus.subscribe::<domain::audio_file::AudioFileEvent>(Arc::new(genre_stats_handler_audio))
        .await;
    bus.subscribe::<domain::album::AlbumEvent>(Arc::new(genre_stats_handler_album))
        .await;

    bus.subscribe::<domain::audio_file::AudioFileEvent>(Arc::new(scan_status_handler))
        .await;
    bus.subscribe::<domain::library::LibraryEvent>(Arc::new(scan_lifecycle_handler))
        .await;
    bus.subscribe::<domain::annotation::AnnotationEvent>(Arc::new(playback_history_handler))
        .await;
}
