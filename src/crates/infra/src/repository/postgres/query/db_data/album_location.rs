use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};

use domain::value::{AlbumId, MediaPath};
use model::album_location::AlbumLocation;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "album_location")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[sea_orm(column_type = "BigInteger")]
    #[sea_orm(auto_increment = true)]
    pub id: i64,

    #[sea_orm(column_type = "BigInteger")]
    pub album_id: i64,

    pub location_protocol: String,
    pub location_path: String,

    pub total: i32,

    pub update_time: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No RelationDef")
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for AlbumLocation {
    fn from(model: Model) -> Self {
        AlbumLocation {
            album_id: AlbumId::from(model.album_id),
            location: MediaPath::new(model.location_protocol, model.location_path),
            total: model.total,
            update_time: model.update_time,
        }
    }
}

impl From<&AlbumLocation> for ActiveModel {
    fn from(entry: &AlbumLocation) -> Self {
        Self {
            id: Default::default(),
            album_id: Set(i64::from(entry.album_id.clone())),
            location_protocol: Set(entry.location.protocol.clone()),
            location_path: Set(entry.location.path.clone()),
            total: Set(entry.total),
            update_time: Set(entry.update_time),
        }
    }
}
