use domain::playlist::{Owner, Playlist};
use domain::value::{PlaylistId, UserId};
use sea_orm::entity::prelude::*;
use sea_orm::Set;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "playlist")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[sea_orm(column_type = "BigInteger")]
    pub id: i64,
    pub name: String,
    pub comment: Option<String>,
    #[sea_orm(column_type = "BigInteger")]
    pub owner_id: i64,
    pub owner_name: String,
    pub public: bool,
    #[sea_orm(column_type = "BigInteger")]
    pub version: i64,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    PlaylistEntry,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::PlaylistEntry => Entity::has_many(super::playlist_entry::Entity)
                .from(Column::Id)
                .to(super::playlist_entry::Column::PlaylistId)
                .into(),
        }
    }
}

impl Related<super::playlist_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PlaylistEntry.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<&Playlist> for ActiveModel {
    fn from(playlist: &Playlist) -> Self {
        Self {
            id: Set(playlist.id.as_i64()),
            name: Set(playlist.name.clone()),
            comment: Set(playlist.comment.clone()),
            owner_id: Set(playlist.owner.id.as_i64()),
            owner_name: Set(playlist.owner.name.clone()),
            public: Set(playlist.public),
            version: Set(playlist.version),
            created_at: Set(playlist.created_at),
            updated_at: Set(playlist.updated_at),
        }
    }
}

impl From<Model> for Playlist {
    fn from(model: Model) -> Self {
        Playlist {
            id: PlaylistId::from(model.id),
            name: model.name,
            comment: model.comment,
            owner: Owner {
                id: UserId::from(model.owner_id),
                name: model.owner_name,
            },
            public: model.public,
            entries: Vec::new(),
            created_at: model.created_at,
            updated_at: model.updated_at,
            version: model.version,
            deleted: false,
        }
    }
}
