use application::query::dao::PlaylistDao;
use application::query::QueryError;
use async_trait::async_trait;
use model::playlist::{Playlist, PlaylistAudioFile, PlaylistSummary, PlaylistTrack};
use sea_orm::*;

pub struct PlaylistDaoImpl {
    db: DatabaseConnection,
}

impl PlaylistDaoImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[derive(Debug, Clone, FromQueryResult)]
struct PlaylistRow {
    pub id: i64,
    pub name: String,
    pub comment: String,
    pub owner_id: i64,
    pub owner_name: String,
    pub public: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub song_count: i64,
    pub duration: i64,
}

impl From<PlaylistRow> for PlaylistSummary {
    fn from(row: PlaylistRow) -> Self {
        PlaylistSummary {
            id: row.id,
            name: row.name,
            comment: if row.comment.is_empty() {
                None
            } else {
                Some(row.comment)
            },
            duration: row.duration as i32,
            song_count: row.song_count as i32,
            owner_id: row.owner_id,
            owner_name: row.owner_name,
            public: row.public == 1,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromQueryResult)]
struct PlaylistEntryRow {
    pub entry_id: i64,
    // AudioFile fields
    pub id: i64,
    pub title: String,
    pub album_id: Option<i64>,
    pub album_name: Option<String>,
    pub artist_id: Option<i64>,
    pub artist_name: Option<String>,
    pub duration: Option<i64>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub bit_rate: Option<i32>,
    pub size: Option<i64>,
    pub suffix: Option<String>,
    pub content_type: Option<String>,
    pub path: String,
    pub cover_art_id: Option<i64>,
    pub play_count: i32,
    pub starred: Option<bool>,
    pub rating: Option<i32>,
    pub created_at: i64,
}

#[async_trait]
impl PlaylistDao for PlaylistDaoImpl {
    async fn get_by_id(&self, id: i64) -> Result<Option<Playlist>, QueryError> {
        // 首先获取播放列表基本信息
        let playlist_row: Option<PlaylistRow> = PlaylistRow::find_by_statement(
            Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                SELECT 
                    p.id, p.name, COALESCE(p.comment, '') as comment, 
                    p.owner_id, p.owner_name, 
                    CASE WHEN p.public THEN 1 ELSE 0 END as public,
                    EXTRACT(EPOCH FROM p.created_at)::bigint as created_at,
                    EXTRACT(EPOCH FROM p.updated_at)::bigint as updated_at,
                    COUNT(pe.id) as song_count,
                    COALESCE(SUM(af.duration), 0)::bigint as duration
                FROM playlist p
                LEFT JOIN playlist_entry pe ON p.id = pe.playlist_id
                LEFT JOIN audio_file af ON pe.audio_file_id = af.id
                WHERE p.id = $1
                GROUP BY p.id
                "#,
                vec![id.into()],
            ),
        )
        .one(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        let playlist_row = match playlist_row {
            Some(row) => row,
            None => return Ok(None),
        };

        // 获取播放列表中的歌曲
        let entry_rows: Vec<PlaylistEntryRow> = PlaylistEntryRow::find_by_statement(
            Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                SELECT 
                    pe.id as entry_id,
                    af.id,
                    af.title,
                    af.album_id,
                    al.name as album_name,
                    af.artist_id,
                    ar.name as artist_name,
                    af.duration,
                    af.track_number,
                    af.disc_number,
                    af.year,
                    g.name as genre,
                    af.bit_rate,
                    af.size,
                    af.suffix,
                    NULL::text as content_type,
                    (af.path_protocol || '://' || af.path_path) as path,
                    NULL::bigint as cover_art_id,
                    COALESCE(ann.played_count, 0) as play_count,
                    COALESCE(ann.starred, false) as starred,
                    ann.rating,
                    EXTRACT(EPOCH FROM af.created_at)::bigint as created_at
                FROM playlist_entry pe
                JOIN audio_file af ON pe.audio_file_id = af.id
                LEFT JOIN album al ON af.album_id = al.id
                LEFT JOIN artist ar ON af.artist_id = ar.id
                LEFT JOIN genre g ON af.genre_id = g.id
                LEFT JOIN annotation ann ON ann.item_id = af.id AND ann.item_kind = 'audio_file'
                WHERE pe.playlist_id = $1
                ORDER BY pe.added_at
                "#,
                vec![id.into()],
            ),
        )
        .all(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        let tracks: Vec<PlaylistTrack> = entry_rows
            .into_iter()
            .map(|row| PlaylistTrack {
                id: row.entry_id,
                audio_file: PlaylistAudioFile {
                    id: row.id,
                    title: row.title,
                    album_id: row.album_id,
                    album_name: row.album_name,
                    artist_id: row.artist_id,
                    artist_name: row.artist_name,
                    duration: row.duration,
                    track_number: row.track_number,
                    disc_number: row.disc_number,
                    year: row.year,
                    genre: row.genre,
                    bit_rate: row.bit_rate,
                    size: row.size,
                    suffix: row.suffix,
                    content_type: row.content_type,
                    path: row.path,
                    cover_art_id: row.cover_art_id,
                    play_count: row.play_count,
                    starred: row.starred,
                    rating: row.rating,
                    created_at: row.created_at,
                },
            })
            .collect();

        Ok(Some(Playlist {
            id: playlist_row.id,
            name: playlist_row.name,
            comment: if playlist_row.comment.is_empty() {
                None
            } else {
                Some(playlist_row.comment)
            },
            duration: playlist_row.duration as i32,
            song_count: playlist_row.song_count as i32,
            owner_id: playlist_row.owner_id,
            owner_name: playlist_row.owner_name,
            public: playlist_row.public == 1,
            tracks,
            created_at: playlist_row.created_at,
            updated_at: playlist_row.updated_at,
        }))
    }

    async fn get_by_owner_id(&self, owner_id: i64) -> Result<Vec<PlaylistSummary>, QueryError> {
        let rows: Vec<PlaylistRow> = PlaylistRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
            SELECT 
                p.id, p.name, COALESCE(p.comment, '') as comment, 
                p.owner_id, p.owner_name, 
                CASE WHEN p.public THEN 1 ELSE 0 END as public,
                EXTRACT(EPOCH FROM p.created_at)::bigint as created_at,
                EXTRACT(EPOCH FROM p.updated_at)::bigint as updated_at,
                COUNT(pe.id) as song_count,
                COALESCE(SUM(af.duration), 0)::bigint as duration
            FROM playlist p
            LEFT JOIN playlist_entry pe ON p.id = pe.playlist_id
            LEFT JOIN audio_file af ON pe.audio_file_id = af.id
            WHERE p.owner_id = $1
            GROUP BY p.id
            ORDER BY p.updated_at DESC
            "#,
            vec![owner_id.into()],
        ))
        .all(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(rows.into_iter().map(|row| row.into()).collect())
    }
}
