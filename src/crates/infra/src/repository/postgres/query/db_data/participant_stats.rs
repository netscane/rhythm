use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::{NotSet, Set};
use serde::{Deserialize, Serialize};

use domain::value::{ArtistId, ParticipantRole, ParticipantWorkType};
use model::participant_stats::ParticipantStats;

use crate::repository::postgres::command::db_data::artist;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "participant_stats")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[sea_orm(column_type = "BigInteger")]
    pub id: i64,

    #[sea_orm(column_type = "BigInteger")]
    pub artist_id: i64,

    #[sea_orm(column_type = "Text")]
    pub role: String,

    pub duration: i64,

    #[sea_orm(column_type = "BigInteger")]
    pub size: i64,

    #[sea_orm(column_type = "BigInteger")]
    pub song_count: i32,

    #[sea_orm(column_type = "BigInteger")]
    pub album_count: i32,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Artist,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Artist => Entity::belongs_to(artist::Entity)
                .from(Column::ArtistId)
                .to(artist::Column::Id)
                .into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for ParticipantStats {
    fn from(model: Model) -> Self {
        ParticipantStats {
            role: ParticipantRole::from(model.role.as_str()),
            artist_id: ArtistId::from(model.artist_id),
            duration: model.duration,
            size: model.size,
            song_count: model.song_count,
            album_count: model.album_count,
        }
    }
}

impl From<&ParticipantStats> for ActiveModel {
    fn from(participant_stats: &ParticipantStats) -> Self {
        Self {
            id: NotSet,
            artist_id: Set(i64::from(participant_stats.artist_id.clone())),
            role: Set(participant_stats.role.to_string()),
            duration: Set(participant_stats.duration),
            size: Set(participant_stats.size),
            song_count: Set(participant_stats.song_count),
            album_count: Set(participant_stats.album_count),
        }
    }
}
