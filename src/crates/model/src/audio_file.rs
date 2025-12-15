use super::shared::{Annotation, ArtistSummary, Contributor, GenreSummary};
use chrono::NaiveDateTime;

#[derive(Debug)]
pub struct AudioFile {
    pub id: i64,
    pub library_id: i32,
    pub path: String,
    pub title: String,
    pub album: String,
    pub artists: Vec<ArtistSummary>,
    pub album_artists: Vec<ArtistSummary>,
    pub album_id: i64,
    pub has_cover_art: bool,
    pub track_number: i32,
    pub disc_number: i32,
    pub disc_subtitle: String,
    pub year: Option<i32>,
    pub size: i64,
    pub suffix: String,
    pub duration: i64,
    pub bit_rate: i32,
    pub channels: i32,
    pub order_title: String,
    pub bpm: i32,

    pub name: String,
    pub song_count: i32,

    pub compilation: bool,
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
