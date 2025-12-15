use crate::command::shared::IdGenerator;
use crate::context::AppContext;
use crate::error::AppError;
use crate::event::event_bus::{EventBus, EventEnvelope};
use domain::genre::{Genre, GenreEvent, GenreFound, GenreName, GenreRepository};
use std::sync::Arc;

#[derive(Debug)]
pub struct CreateGenreCmd {
    pub name: String,
}

#[derive(Clone)]
pub struct GenreService<B: EventBus> {
    id_generator: Arc<dyn IdGenerator>,
    genre_repository: Arc<dyn GenreRepository>,
    event_bus: Arc<B>,
}

impl<B: EventBus> GenreService<B> {
    pub fn new(
        id_generator: Arc<dyn IdGenerator>,
        genre_repository: Arc<dyn GenreRepository>,
        event_bus: Arc<B>,
    ) -> Self {
        Self {
            id_generator,
            genre_repository,
            event_bus,
        }
    }

    pub async fn create_genre(
        &self,
        context: &AppContext,
        cmd: CreateGenreCmd,
    ) -> Result<Genre, AppError> {
        let sort_name = GenreName::new(cmd.name)?;
        let existing_genre = self.genre_repository.find_by_name(&sort_name).await?;

        if let Some(genre) = existing_genre {
            // 如果找到了现有的 genre，发布 Found 事件并返回它
            let event = GenreEvent::Found(GenreFound {
                genre_id: genre.id.clone(),
                version: genre.version,
            });
            let envelope = EventEnvelope::new(
                genre.id.as_i64(),
                genre.version,
                event,
                context.correlation_id.clone(),
                context.causation_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
            return Ok(genre);
        }

        // 只有在没有找到现有 genre 时才创建新的
        let genre_id = self.id_generator.next_id().await?;
        let mut genre = Genre::new(genre_id.into(), sort_name)?;
        let events = genre.take_events();
        let genre = self.genre_repository.save(genre).await?;

        // 发布领域事件
        for event in events {
            let envelope = EventEnvelope::new(
                genre.id.as_i64(),
                genre.version,
                event,
                context.correlation_id.clone(),
                context.causation_id.clone(),
            );
            self.event_bus.publish(envelope).await?;
        }

        Ok(genre)
    }
}
