use application::query::dao::PlayQueueDao;
use application::query::QueryError;
use async_trait::async_trait;
use model::play_queue::PlayQueue;
use model::playlist::PlaylistAudioFile;
use sea_orm::*;

pub struct PlayQueueDaoImpl {
    db: DatabaseConnection,
}

impl PlayQueueDaoImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[derive(Debug, Clone, FromQueryResult)]
struct PlayQueueRow {
    pub id: i64,
    pub current_id: Option<i64>,
    pub position: i64,
    pub changed_by: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, FromQueryResult)]
struct PlayQueueEntryRow {
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
impl PlayQueueDao for PlayQueueDaoImpl {
    async fn get_by_user_id(
        &self,
        user_id: i64,
        username: &str,
    ) -> Result<Option<PlayQueue>, QueryError> {
        // Get play queue basic info
        let queue_row: Option<PlayQueueRow> = PlayQueueRow::find_by_statement(
            Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                SELECT 
                    pq.id,
                    pq.current_id,
                    pq.position,
                    pq.changed_by,
                    EXTRACT(EPOCH FROM pq.updated_at)::bigint as updated_at
                FROM play_queue pq
                WHERE pq.user_id = $1
                "#,
                vec![user_id.into()],
            ),
        )
        .one(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        let queue_row = match queue_row {
            Some(row) => row,
            None => return Ok(None),
        };

        // Get entries with audio file details
        let entry_rows: Vec<PlayQueueEntryRow> = PlayQueueEntryRow::find_by_statement(
            Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                SELECT 
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
                FROM play_queue_item pqi
                JOIN audio_file af ON pqi.audio_file_id = af.id
                LEFT JOIN album al ON af.album_id = al.id
                LEFT JOIN artist ar ON af.artist_id = ar.id
                LEFT JOIN genre g ON af.genre_id = g.id
                LEFT JOIN annotation ann ON ann.item_id = af.id AND ann.item_kind = 'audio_file'
                WHERE pqi.play_queue_id = $1
                ORDER BY pqi.position
                "#,
                vec![queue_row.id.into()],
            ),
        )
        .all(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        let entries: Vec<PlaylistAudioFile> = entry_rows
            .into_iter()
            .map(|row| PlaylistAudioFile {
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
            })
            .collect();

        // Format updated_at as ISO 8601
        let changed = chrono::DateTime::from_timestamp(queue_row.updated_at, 0)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_default();

        Ok(Some(PlayQueue {
            current_id: queue_row.current_id,
            position: queue_row.position,
            username: username.to_string(),
            changed_by: queue_row.changed_by,
            changed,
            entries,
        }))
    }
}
