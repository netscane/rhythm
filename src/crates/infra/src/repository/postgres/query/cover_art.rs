use application::query::dao::{
    CoverArtDao, CoverArtInfo, CoverArtPath, CoverArtPathWithSource, LocationInfo,
};
use application::query::QueryError;
use async_trait::async_trait;
use sea_orm::*;

pub struct CoverArtDaoImpl {
    db: DatabaseConnection,
}

impl CoverArtDaoImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

/// 音频文件基础信息
#[derive(Debug, Clone, FromQueryResult)]
struct AudioFileCoverRow {
    pub path_protocol: String,
    pub path_path: String,
    pub has_cover_art: bool,
    pub updated_at: chrono::NaiveDateTime,
}

/// 专辑基础信息
#[derive(Debug, Clone, FromQueryResult)]
struct AlbumCoverRow {
    pub update_time: chrono::NaiveDateTime,
}

/// 艺术家基础信息
#[derive(Debug, Clone, FromQueryResult)]
struct ArtistCoverRow {
    pub update_time: chrono::NaiveDateTime,
}

/// 位置统计
#[derive(Debug, Clone, FromQueryResult)]
struct LocationRow {
    pub location_protocol: String,
    pub location_path: String,
    pub total: i32,
}

/// 播放列表第一首歌信息
#[derive(Debug, Clone, FromQueryResult)]
struct PlaylistFirstSongRow {
    pub path_protocol: String,
    pub path_path: String,
    pub has_cover_art: bool,
    pub updated_at: chrono::NaiveDateTime,
}

/// 封面路径信息
#[derive(Debug, Clone, FromQueryResult)]
struct CoverArtPathRow {
    pub path_protocol: String,
    pub path_path: String,
}

#[async_trait]
impl CoverArtDao for CoverArtDaoImpl {
    async fn get_audio_file_cover_info(
        &self,
        audio_file_id: i64,
    ) -> Result<Option<CoverArtInfo>, QueryError> {
        let sql = r#"
            SELECT path_protocol, path_path, has_cover_art, updated_at
            FROM audio_file
            WHERE id = $1
        "#;

        let row: Option<AudioFileCoverRow> = AudioFileCoverRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [audio_file_id.into()]),
        )
        .one(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(row.map(|r| CoverArtInfo {
            path: format!("{}://{}", r.path_protocol, r.path_path),
            updated_at: r.updated_at,
            has_embedded: r.has_cover_art,
        }))
    }

    async fn get_album_cover_info(
        &self,
        album_id: i64,
    ) -> Result<Option<CoverArtInfo>, QueryError> {
        let sql = r#"
            SELECT update_time
            FROM album
            WHERE id = $1
        "#;

        let row: Option<AlbumCoverRow> = AlbumCoverRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [album_id.into()]),
        )
        .one(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(row.map(|r| CoverArtInfo {
            path: String::new(), // Will be resolved from locations
            updated_at: r.update_time,
            has_embedded: false,
        }))
    }

    async fn get_album_locations(&self, album_id: i64) -> Result<Vec<LocationInfo>, QueryError> {
        let sql = r#"
            SELECT location_protocol, location_path, total
            FROM album_location
            WHERE album_id = $1 AND total > 0
            ORDER BY total DESC
        "#;

        let rows: Vec<LocationRow> = LocationRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [album_id.into()]),
        )
        .all(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| LocationInfo {
                protocol: r.location_protocol,
                path: r.location_path,
                file_count: r.total,
            })
            .collect())
    }

    async fn get_artist_cover_info(
        &self,
        artist_id: i64,
    ) -> Result<Option<CoverArtInfo>, QueryError> {
        let sql = r#"
            SELECT update_time
            FROM artist
            WHERE id = $1
        "#;

        let row: Option<ArtistCoverRow> = ArtistCoverRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [artist_id.into()]),
        )
        .one(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(row.map(|r| CoverArtInfo {
            path: String::new(), // Will be resolved from locations
            updated_at: r.update_time,
            has_embedded: false,
        }))
    }

    async fn get_artist_album_locations(
        &self,
        artist_id: i64,
        limit: i32,
    ) -> Result<Vec<LocationInfo>, QueryError> {
        // 查询艺术家关联专辑的 album_location
        let sql = r#"
            SELECT DISTINCT al.location_protocol, al.location_path, al.total
            FROM album_location al
            JOIN album a ON al.album_id = a.id
            WHERE a.album_artist_id = $1 AND al.total > 0
            ORDER BY al.location_path DESC
            LIMIT $2
        "#;

        let rows: Vec<LocationRow> =
            LocationRow::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [artist_id.into(), limit.into()],
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| LocationInfo {
                protocol: r.location_protocol,
                path: r.location_path,
                file_count: r.total,
            })
            .collect())
    }

    async fn get_playlist_cover_info(
        &self,
        playlist_id: i64,
    ) -> Result<Option<CoverArtInfo>, QueryError> {
        // Get the first song in the playlist
        let sql = r#"
            SELECT af.path_protocol, af.path_path, af.has_cover_art, af.updated_at
            FROM playlist_entry pe
            JOIN audio_file af ON pe.audio_file_id = af.id
            WHERE pe.playlist_id = $1
            ORDER BY pe.position ASC
            LIMIT 1
        "#;

        let row: Option<PlaylistFirstSongRow> = PlaylistFirstSongRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [playlist_id.into()]),
        )
        .one(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(row.map(|r| CoverArtInfo {
            path: format!("{}://{}", r.path_protocol, r.path_path),
            updated_at: r.updated_at,
            has_embedded: r.has_cover_art,
        }))
    }

    async fn get_cover_art_paths_by_album(
        &self,
        album_id: i64,
    ) -> Result<Vec<CoverArtPath>, QueryError> {
        let sql = r#"
            SELECT path_protocol, path_path
            FROM cover_art
            WHERE album_id = $1
            ORDER BY path_path ASC
        "#;

        let rows: Vec<CoverArtPathRow> = CoverArtPathRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [album_id.into()]),
        )
        .all(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| CoverArtPath {
                protocol: r.path_protocol,
                path: r.path_path,
            })
            .collect())
    }

    async fn get_cover_art_paths_by_prefix(
        &self,
        protocol: &str,
        path_prefix: &str,
    ) -> Result<Vec<CoverArtPath>, QueryError> {
        let sql = r#"
            SELECT path_protocol, path_path
            FROM cover_art
            WHERE path_protocol = $1 AND path_path LIKE $2
            ORDER BY path_path ASC
            limit 10
        "#;

        let like_pattern = format!("{}%", path_prefix);
        let rows: Vec<CoverArtPathRow> =
            CoverArtPathRow::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [protocol.into(), like_pattern.into()],
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| CoverArtPath {
                protocol: r.path_protocol,
                path: r.path_path,
            })
            .collect())
    }

    async fn get_cover_art_by_audio_file(
        &self,
        audio_file_id: i64,
    ) -> Result<Option<CoverArtPathWithSource>, QueryError> {
        #[derive(Debug, Clone, FromQueryResult)]
        struct CoverArtPathWithSourceRow {
            pub path_protocol: String,
            pub path_path: String,
            pub source: String,
        }

        let sql = r#"
            SELECT path_protocol, path_path, source
            FROM cover_art
            WHERE audio_file_id = $1
            LIMIT 1
        "#;

        let row: Option<CoverArtPathWithSourceRow> = CoverArtPathWithSourceRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [audio_file_id.into()]),
        )
        .one(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(row.map(|r| CoverArtPathWithSource {
            protocol: r.path_protocol,
            path: r.path_path,
            source: r.source,
        }))
    }

    async fn get_album_audio_file_covers(
        &self,
        album_id: i64,
    ) -> Result<Vec<CoverArtPathWithSource>, QueryError> {
        #[derive(Debug, Clone, FromQueryResult)]
        struct CoverArtPathWithSourceRow {
            pub path_protocol: String,
            pub path_path: String,
            pub source: String,
        }

        // 查询专辑音频文件的所有封面
        let sql = r#"
            SELECT ca.path_protocol, ca.path_path, ca.source
            FROM cover_art ca
            JOIN audio_file af ON ca.audio_file_id = af.id
            WHERE af.album_id = $1
            limit 3
        "#;

        let rows: Vec<CoverArtPathWithSourceRow> = CoverArtPathWithSourceRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [album_id.into()]),
        )
        .all(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| CoverArtPathWithSource {
                protocol: r.path_protocol,
                path: r.path_path,
                source: r.source,
            })
            .collect())
    }

    async fn get_cover_art_by_artist(
        &self,
        artist_id: i64,
    ) -> Result<Option<CoverArtPath>, QueryError> {
        let sql = r#"
            SELECT path_protocol, path_path
            FROM cover_art
            WHERE artist_id = $1
            LIMIT 1
        "#;

        let row: Option<CoverArtPathRow> = CoverArtPathRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [artist_id.into()]),
        )
        .one(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(row.map(|r| CoverArtPath {
            protocol: r.path_protocol,
            path: r.path_path,
        }))
    }

    async fn get_first_album_id_by_artist(
        &self,
        artist_id: i64,
    ) -> Result<Option<i64>, QueryError> {
        #[derive(Debug, Clone, FromQueryResult)]
        struct AlbumIdRow {
            id: i64,
        }

        let sql = r#"
            SELECT id
            FROM album
            WHERE album_artist_id = $1
            ORDER BY name ASC
            LIMIT 1
        "#;

        let row: Option<AlbumIdRow> = AlbumIdRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [artist_id.into()]),
        )
        .one(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(row.map(|r| r.id))
    }

    async fn get_artist_audio_file_covers(
        &self,
        artist_id: i64,
    ) -> Result<Vec<CoverArtPathWithSource>, QueryError> {
        #[derive(Debug, Clone, FromQueryResult)]
        struct CoverArtPathWithSourceRow {
            pub path_protocol: String,
            pub path_path: String,
            pub source: String,
        }

        // 查询艺术家音频文件的所有封面
        let sql = r#"
            SELECT ca.path_protocol, ca.path_path, ca.source
            FROM cover_art ca
            JOIN audio_file af ON ca.audio_file_id = af.id
            JOIN album a ON af.album_id = a.id
            WHERE a.album_artist_id = $1
        "#;

        let rows: Vec<CoverArtPathWithSourceRow> = CoverArtPathWithSourceRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [artist_id.into()]),
        )
        .all(&self.db)
        .await
        .map_err(|e| QueryError::DbError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| CoverArtPathWithSource {
                protocol: r.path_protocol,
                path: r.path_path,
                source: r.source,
            })
            .collect())
    }
}
