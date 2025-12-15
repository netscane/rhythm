use super::shared::{Annotation, ArtistSummary, Contributor, GenreSummary};
use chrono::NaiveDateTime;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AlbumError {
    #[error("Required parameter is missing:{0}")]
    MissingParameter(String),
    #[error("Operate type:{0} not implemented")]
    OpTypeNotImplemented(String),
    #[error("{0}")]
    DbErr(String),
    #[error(transparent)]
    OtherErr(anyhow::Error),
}

pub type Discs = HashMap<i32, String>;

#[derive(Debug)]
pub struct Album {
    pub id: i64,
    pub library_id: i32,
    pub name: String,
    pub song_count: i32,
    pub duration: i64,
    pub year: Option<i32>,

    pub compilation: bool,
    pub size: i64,
    pub discs: Discs,
    pub sort_name: String,
    pub order_name: String,

    pub annotation: Annotation,
    pub genre: Option<GenreSummary>,
    pub genres: Vec<GenreSummary>,
    pub artist: ArtistSummary,
    pub contributors: Vec<Contributor>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug)]
pub struct AlbumInfo {
    pub id: i64,
    pub description: Option<String>,
}
