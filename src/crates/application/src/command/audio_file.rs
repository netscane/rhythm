use crate::command::shared::IdGenerator;
use crate::context::AppContext;
use crate::error::AppError;
use crate::event::event_bus::{EventBus, EventEnvelope};
use domain::audio_file::{AudioFile, AudioFileRepository};
use domain::value::{
    AlbumId, ArtistId, AudioFileId, AudioMetadata, FileMeta, GenreId, LibraryId, Participant,
    ParticipantRole, ParticipantSubRole, ParticipantWorkType,
};
use std::sync::Arc;

#[derive(Debug)]
pub struct CreateAudioFileCmd {
    pub filemeta: FileMeta,
    pub audio_metadata: AudioMetadata,
    pub library_id: LibraryId,
}

#[derive(Debug)]
pub struct BindGenreToAudioFileCmd {
    pub genre_id: GenreId,
    pub audio_file_id: AudioFileId,
}

#[derive(Debug)]
pub struct BindArtistToAudioFileCmd {
    pub artist_id: ArtistId,
    pub audio_file_id: AudioFileId,
    pub role: ParticipantRole,
    pub sub_role: Option<ParticipantSubRole>,
}

#[derive(Debug)]
pub struct BindCmd {
    pub audio_file_id: AudioFileId,
    pub album_id: AlbumId,
    pub genre_ids: Vec<GenreId>,
    pub artists: Vec<(ArtistId, ParticipantRole, Option<ParticipantSubRole>)>,
}

#[derive(Clone)]
pub struct AudioFileService<B: EventBus> {
    id_generator: Arc<dyn IdGenerator>,
    audio_file_repository: Arc<dyn AudioFileRepository>,
    event_bus: Arc<B>,
}

impl<B: EventBus> AudioFileService<B> {
    pub fn new(
        id_generator: Arc<dyn IdGenerator>,
        audio_file_repository: Arc<dyn AudioFileRepository>,
        event_bus: Arc<B>,
    ) -> Self {
        Self {
            id_generator,
            audio_file_repository,
            event_bus,
        }
    }

    pub async fn create_audio_file(
        &self,
        context: &AppContext,
        cmd: CreateAudioFileCmd,
    ) -> Result<AudioFile, AppError> {
        let mut audio_file = AudioFile::new(
            self.id_generator.next_id().await?.into(),
            cmd.library_id.clone(),
            cmd.filemeta.path,
            cmd.filemeta.size,
            cmd.filemeta.suffix.clone(),
            cmd.filemeta.hash.clone(),
            cmd.audio_metadata.duration,
            cmd.audio_metadata.bit_rate,
            0,
            cmd.audio_metadata.sample_rate,
            cmd.audio_metadata.channels,
            if let Some(_) = &cmd.audio_metadata.picture {
                true
            } else {
                false
            },
            cmd.audio_metadata.into(),
        );

        let events = audio_file.take_events();
        let audio_file = self.audio_file_repository.save(audio_file).await?;
        for event in events {
            let envelope = EventEnvelope::new(
                audio_file.id.as_i64(),
                audio_file.version,
                event,
                context.correlation_id.clone(),
                context.causation_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(audio_file)
    }

    pub async fn bind(&self, context: &AppContext, cmd: BindCmd) -> Result<(), AppError> {
        let mut audio_file = self
            .audio_file_repository
            .find_by_id(&cmd.audio_file_id)
            .await?
            .expect("AudioFile not found");
        /*
        .ok_or_else(|| {
            AppError::AggregateNotFound("AudioFile".to_string(), cmd.audio_file_id.to_string())
        })?;
        */

        // 绑定流派
        for genre_id in cmd.genre_ids {
            audio_file.bind_to_genre(genre_id)?;
        }

        // 绑定艺术家
        for (artist_id, role, sub_role) in cmd.artists {
            let participant = Participant::new(
                artist_id,
                role,
                sub_role,
                cmd.audio_file_id.as_i64(),
                ParticipantWorkType::Artist,
            );
            audio_file.add_participant(participant)?;
        }

        // 绑定专辑
        audio_file.bind_to_album(cmd.album_id)?;

        // 保存并发布事件
        let events = audio_file.take_events();
        let audio_file = self.audio_file_repository.save(audio_file).await?;

        for event in events {
            let envelope = EventEnvelope::new(
                audio_file.id.as_i64(),
                audio_file.version,
                event,
                context.correlation_id.clone(),
                context.event_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }
        Ok(())
    }
}
