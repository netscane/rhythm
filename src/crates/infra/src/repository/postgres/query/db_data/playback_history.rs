use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::{NotSet, Set};
use serde::{Deserialize, Serialize};

use domain::value::{AudioFileId, PlayerId, UserId};
use model::playback_history::PlaybackHistoryEntry;

use crate::repository::postgres::command::db_data::audio_file;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "playback_history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    #[sea_orm(column_type = "BigInteger")]
    pub id: i64,

    #[sea_orm(column_type = "BigInteger")]
    pub user_id: i64,
    #[sea_orm(column_type = "BigInteger")]
    pub audio_file_id: i64,
    pub scrobbled_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    AudioFile,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::AudioFile => Entity::belongs_to(audio_file::Entity)
                .from(Column::AudioFileId)
                .to(audio_file::Column::Id)
                .into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for PlaybackHistoryEntry {
    fn from(model: Model) -> Self {
        PlaybackHistoryEntry {
            user_id: UserId::from(model.user_id),
            audio_file_id: AudioFileId::from(model.audio_file_id),
            scrobbled_at: model.scrobbled_at,
        }
    }
}

impl From<&PlaybackHistoryEntry> for ActiveModel {
    fn from(entry: &PlaybackHistoryEntry) -> Self {
        Self {
            id: NotSet,
            user_id: Set(entry.user_id.as_i64()),
            audio_file_id: Set(i64::from(entry.audio_file_id.clone())),
            scrobbled_at: Set(entry.scrobbled_at),
        }
    }
}
