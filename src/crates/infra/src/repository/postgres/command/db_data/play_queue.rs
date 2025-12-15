use domain::play_queue::PlayQueue;
use domain::value::{AudioFileId, PlayQueueId, UserId};
use sea_orm::entity::prelude::*;
use sea_orm::Set;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "play_queue")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[sea_orm(column_type = "BigInteger")]
    pub id: i64,
    #[sea_orm(column_type = "BigInteger")]
    pub user_id: i64,
    /// Current playing audio file id
    #[sea_orm(column_type = "BigInteger", nullable)]
    pub current_id: Option<i64>,
    /// Position in milliseconds within the currently playing song
    #[sea_orm(column_type = "BigInteger")]
    pub position: i64,
    /// Client name that last changed this queue
    pub changed_by: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    PlayQueueItem,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::PlayQueueItem => Entity::has_many(super::play_queue_item::Entity)
                .from(Column::Id)
                .to(super::play_queue_item::Column::PlayQueueId)
                .into(),
        }
    }
}

impl Related<super::play_queue_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PlayQueueItem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<&PlayQueue> for ActiveModel {
    fn from(pq: &PlayQueue) -> Self {
        let current_id = pq.current_index.and_then(|idx| {
            pq.items.get(idx).map(|id| id.as_i64())
        });
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: Set(pq.id.as_i64()),
            user_id: Set(pq.user_id.as_i64()),
            current_id: Set(current_id),
            position: Set(pq.position),
            changed_by: Set(pq.changed_by.clone()),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }
}

impl Model {
    pub fn into_play_queue(self, items: Vec<AudioFileId>) -> PlayQueue {
        let current_index = self.current_id.and_then(|cid| {
            items.iter().position(|id| id.as_i64() == cid)
        });
        PlayQueue {
            id: PlayQueueId::from(self.id),
            name: String::new(),
            user_id: UserId::from(self.user_id),
            items,
            current_index,
            position: self.position,
            changed_by: self.changed_by,
            updated_at: self.updated_at.and_utc().timestamp(),
            pending_events: vec![],
        }
    }
}
