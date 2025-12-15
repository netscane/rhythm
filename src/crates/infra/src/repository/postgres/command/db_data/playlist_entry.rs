use chrono::Utc;
use domain::playlist::PlaylistEntry;
use domain::value::PlaylistId;
use sea_orm::{
    entity::prelude::*,
    ActiveModelBehavior,
    ActiveValue::{NotSet, Set},
};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "playlist_entry")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[sea_orm(column_type = "BigInteger")]
    pub id: i64,
    #[sea_orm(column_type = "BigInteger")]
    pub playlist_id: i64,
    #[sea_orm(column_type = "BigInteger")]
    pub audio_file_id: i64,
    pub position: i32,
    pub added_at: DateTime,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Playlist,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Playlist => Entity::belongs_to(super::playlist::Entity)
                .from(Column::PlaylistId)
                .to(super::playlist::Column::Id)
                .into(),
        }
    }
}

impl Related<super::playlist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Playlist.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<&PlaylistEntry> for ActiveModel {
    fn from(entry: &PlaylistEntry) -> Self {
        let now = Utc::now().naive_utc();
        ActiveModel {
            id: Set(entry.id),
            playlist_id: Set(entry.playlist_id.as_i64()),
            audio_file_id: Set(entry.audio_file_id),
            position: Set(entry.position),
            added_at: Set(entry.added_at),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }
}

impl From<Model> for PlaylistEntry {
    fn from(model: Model) -> Self {
        PlaylistEntry {
            id: model.id,
            playlist_id: PlaylistId::from(model.playlist_id),
            audio_file_id: model.audio_file_id,
            position: model.position,
            added_at: model.added_at,
        }
    }
}
