use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};

use domain::value::GenreId;
use model::genre::GenreStats;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "genre_stats")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[sea_orm(column_type = "BigInteger")]
    pub genre_id: i64,

    pub song_count: i32,

    pub album_count: i32,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No relations defined")
    }
}

impl From<Model> for GenreStats {
    fn from(model: Model) -> Self {
        GenreStats {
            genre_id: GenreId::from(model.genre_id),
            song_count: model.song_count,
            album_count: model.album_count,
        }
    }
}

impl From<&GenreStats> for ActiveModel {
    fn from(genre_stats: &GenreStats) -> Self {
        Self {
            genre_id: Set(i64::from(genre_stats.genre_id.clone())),
            song_count: Set(genre_stats.song_count),
            album_count: Set(genre_stats.album_count),
        }
    }
}
