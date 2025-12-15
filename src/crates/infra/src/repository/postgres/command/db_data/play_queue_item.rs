use domain::value::AudioFileId;
use sea_orm::{
    entity::prelude::*,
    ActiveModelBehavior,
    ActiveValue::Set,
};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "play_queue_item")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[sea_orm(column_type = "BigInteger")]
    pub id: i64,
    #[sea_orm(column_type = "BigInteger")]
    pub play_queue_id: i64,
    #[sea_orm(column_type = "BigInteger")]
    pub audio_file_id: i64,
    /// Position in the queue (0-based)
    pub position: i32,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    PlayQueue,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::PlayQueue => Entity::belongs_to(super::play_queue::Entity)
                .from(Column::PlayQueueId)
                .to(super::play_queue::Column::Id)
                .into(),
        }
    }
}

impl Related<super::play_queue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PlayQueue.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// Item for batch insert
pub struct PlayQueueItemInsert {
    pub id: i64,
    pub play_queue_id: i64,
    pub audio_file_id: i64,
    pub position: i32,
}

impl From<PlayQueueItemInsert> for ActiveModel {
    fn from(item: PlayQueueItemInsert) -> Self {
        let now = chrono::Utc::now().naive_utc();
        ActiveModel {
            id: Set(item.id),
            play_queue_id: Set(item.play_queue_id),
            audio_file_id: Set(item.audio_file_id),
            position: Set(item.position),
            created_at: Set(now),
        }
    }
}

impl From<Model> for AudioFileId {
    fn from(model: Model) -> Self {
        AudioFileId::from(model.audio_file_id)
    }
}
