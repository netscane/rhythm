//! `SeaORM` Entity for CoverArt

use chrono::NaiveDateTime;
use domain::cover_art::{CoverArt, CoverArtDTO, CoverFormat, CoverSourceType};
use domain::value::{AlbumId, AudioFileId, CoverArtId, MediaPath};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::{NotSet, Set};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "cover_art")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub version: i64,

    // Optional relationships
    pub audio_file_id: Option<i64>,
    pub album_id: Option<i64>,

    // File path information
    pub path_protocol: String,
    pub path_path: String,

    // Image metadata (nullable)
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub format: Option<String>, // CoverFormat as string
    pub file_size: i64,
    pub source: String, // CoverSourceType as string

    // Timestamps
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    #[allow(dead_code)]
    None,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::None => panic!("No relations defined for CoverArt"),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<CoverArt> for ActiveModel {
    fn from(cover_art: CoverArt) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: Set(cover_art.id.into()),
            version: Set(cover_art.version),
            audio_file_id: Set(cover_art.audio_file_id.map(|id| id.into())),
            album_id: Set(None),
            path_protocol: Set(cover_art.path.protocol),
            path_path: Set(cover_art.path.path),
            width: Set(cover_art.width),
            height: Set(cover_art.height),
            format: Set(cover_art.format.map(|f| f.to_string())),
            file_size: Set(cover_art.file_size),
            source: Set(cover_art.source.to_string()),
            updated_at: Set(now),
            created_at: Set(now),
        }
    }
}

impl From<Model> for CoverArt {
    fn from(model: Model) -> Self {
        let path = MediaPath {
            protocol: model.path_protocol,
            path: model.path_path,
        };

        let format = model.format.map(|f| match f.as_str() {
            "jpeg" => CoverFormat::Jpeg,
            "png" => CoverFormat::Png,
            "webp" => CoverFormat::WebP,
            "gif" => CoverFormat::Gif,
            "bmp" => CoverFormat::Bmp,
            "tiff" => CoverFormat::Tiff,
            _ => CoverFormat::Jpeg, // default
        });

        let source = match model.source.as_str() {
            "embedded" => CoverSourceType::Embedded,
            "external" => CoverSourceType::External,
            "downloaded" => CoverSourceType::Downloaded,
            "generated" => CoverSourceType::Generated,
            "manual" => CoverSourceType::Manual,
            _ => CoverSourceType::External, // default
        };

        let audio_opt = model.audio_file_id.map(AudioFileId::from);
        let album_opt = model.album_id.map(AlbumId::from);

        let dto = CoverArtDTO {
            audio_file_id: audio_opt,
            album_id: album_opt,
            path: path.clone(),
            width: model.width,
            height: model.height,
            format,
            file_size: model.file_size,
            source: source.clone(),
        };

        let mut cover = CoverArt::from_dto(CoverArtId::from(model.id), dto).unwrap_or_else(|_| {
            // If validation fails, create with minimal valid data
            CoverArt::from_dto(
                CoverArtId::from(model.id),
                CoverArtDTO {
                    audio_file_id: None,
                    album_id: None,
                    path,
                    width: None,
                    height: None,
                    format: None,
                    file_size: 1,
                    source,
                },
            )
            .expect("Failed to create CoverArt with minimal data")
        });

        // Override persistence metadata from the DB record
        cover.version = model.version;

        cover
    }
}
