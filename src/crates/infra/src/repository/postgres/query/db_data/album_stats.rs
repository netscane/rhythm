use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};

use domain::value::AlbumId;
use model::album_stats::AlbumStats;

use crate::repository::postgres::command::db_data::album;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "album_stats")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[sea_orm(column_type = "BigInteger")]
    pub album_id: i64,

    #[sea_orm(column_type = "BigInteger")]
    pub duration: i64,

    #[sea_orm(column_type = "BigInteger")]
    pub size: i64,

    #[sea_orm(column_type = "BigInteger")]
    pub song_count: i32,

    pub disk_numbers: Vec<i32>,

    pub year: i32,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Album,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Album => Entity::belongs_to(album::Entity)
                .from(Column::AlbumId)
                .to(album::Column::Id)
                .into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for AlbumStats {
    fn from(model: Model) -> Self {
        AlbumStats {
            album_id: AlbumId::from(model.album_id),
            duration: model.duration,
            size: model.size,
            song_count: model.song_count,
            disk_numbers: model.disk_numbers,
            year: if model.year != 0 {
                Some(model.year)
            } else {
                None
            },
        }
    }
}

impl From<&AlbumStats> for ActiveModel {
    fn from(album_stats: &AlbumStats) -> Self {
        Self {
            album_id: Set(i64::from(album_stats.album_id.clone())),
            duration: Set(album_stats.duration),
            size: Set(album_stats.size),
            song_count: Set(album_stats.song_count),
            disk_numbers: Set(album_stats.disk_numbers.clone()),
            year: Set(album_stats.year.unwrap_or(0)),
        }
    }
}
