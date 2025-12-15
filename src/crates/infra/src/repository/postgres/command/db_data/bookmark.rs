use super::song;
use domain::bookmark::{Bookmark, EntityStatus};
use sea_orm::{
    entity::prelude::*,
    ActiveValue::{NotSet, Set, Unchanged},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "bookmark")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[sea_orm(column_type = "BigInteger")]
    pub id: i64,
    #[sea_orm(column_type = "BigInteger")]
    pub song_id: i64,
    pub comment: String,
    pub position: i64,
    #[sea_orm(column_type = "BigInteger")]
    pub changed_by: i64,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    #[sea_orm(column_type = "BigInteger")]
    pub version: i64,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Song,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Song => Entity::belongs_to(song::Entity)
                .from(Column::SongId)
                .to(song::Column::Id)
                .into(),
        }
    }
}

impl Related<song::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Song.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Bookmark> for ActiveModel {
    fn from(value: Bookmark) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            id: if value.status == EntityStatus::New {
                Set(value.id)
            } else {
                Unchanged(value.id)
            },
            song_id: Set(value.song_id),
            comment: Set(value.comment),
            position: Set(value.position),
            changed_by: Set(value.changed_by),
            created_at: if value.status == EntityStatus::New {
                Set(now)
            } else {
                NotSet
            },
            updated_at: Set(now),
            version: if value.status == EntityStatus::New {
                Set(1)
            } else {
                Set(value.version)
            },
        }
    }
}

impl From<Model> for Bookmark {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            song_id: model.song_id,
            comment: model.comment,
            position: model.position,
            changed_by: model.changed_by,
            version: model.version,
            status: EntityStatus::Active,
            old_version: model.version,
        }
    }
}
