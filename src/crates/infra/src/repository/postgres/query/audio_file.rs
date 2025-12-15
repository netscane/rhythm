use std::collections::HashMap;

use application::query::dao::AudioFileDao;
use application::query::QueryError;
use async_trait::async_trait;
use model::audio_file::AudioFile;
use model::shared::{Annotation, ArtistSummary, Contributor, GenreSummary};
use sea_orm::*;

pub struct AudioFileDaoImpl {
    db: DatabaseConnection,
}

impl AudioFileDaoImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

/// AudioFileQueryFilter 音频文件查询过滤器
#[derive(Debug, Clone)]
enum AudioFileQueryFilter {
    ById(i64),
    ByArtistId(i64),
    ByAlbumId(i64),
    ByGenre(String),
    ByYearRange(i32, i32),
    ByStarred(i64), // user_id
    #[allow(dead_code)]
    All,
}

/// AudioFileQueryOrderBy 排序方式
#[derive(Debug, Clone, Default)]
enum AudioFileQueryOrderBy {
    #[default]
    ByTitle,
    ByPlayedCountDesc,
    ByPlayedAtDesc,
    Random,
}

/// AudioFileQueryOptions 查询选项
#[derive(Debug, Clone)]
struct AudioFileQueryOptions {
    filters: Vec<AudioFileQueryFilter>,
    order_by: AudioFileQueryOrderBy,
    limit: Option<i32>,
    offset: Option<i32>,
}

impl Default for AudioFileQueryOptions {
    fn default() -> Self {
        Self {
            filters: vec![],
            order_by: AudioFileQueryOrderBy::ByTitle,
            limit: None,
            offset: None,
        }
    }
}

/// 基础音频文件数据（不含 contributors 和 genres）
#[derive(Debug, Clone, FromQueryResult)]
struct AudioFileBase {
    pub id: i64,
    pub name: String,
    pub sort_name: String,
    pub order_name: String,
    pub compilation: bool,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
    pub year: Option<i32>,
    pub size: i64,
    pub duration: i64,
    pub bit_rate: i32,
    pub suffix: String,
    pub path: String,
    pub bpm: Option<i32>,
    pub channel_count: Option<i32>,
    pub sample_rate: Option<i32>,
    pub has_cover_art: bool,
    pub album_id: i64,
    pub album_name: String,
    pub artist_id: i64,
    pub artist_name: String,
    pub played_count: Option<i32>,
    pub played_at: Option<chrono::NaiveDateTime>,
    pub rating: Option<i32>,
    pub starred: Option<bool>,
    pub starred_at: Option<chrono::NaiveDateTime>,
    pub genre_id: i64,
    pub genre_name: String,
}

/// Participant 数据
#[derive(Debug, Clone, FromQueryResult)]
struct ParticipantRow {
    pub work_id: i64,
    pub artist_id: i64,
    pub artist_name: String,
    pub role: String,
    pub sub_role: Option<String>,
}

/// 副流派数据
#[derive(Debug, Clone, FromQueryResult)]
struct SecondaryGenreRow {
    pub audio_file_id: i64,
    pub genre_id: i64,
    pub genre_name: String,
}

impl AudioFileDaoImpl {
    /// 第一步：查询音频文件基础信息（不含一对多关系）
    fn build_base_query_sql(options: &AudioFileQueryOptions) -> (String, Vec<Value>) {
        let mut values: Vec<Value> = Vec::new();
        let mut param_index = 1;

        // 检查是否需要 participant join（用于 ByArtistId 过滤器）
        let needs_artist_filter = options
            .filters
            .iter()
            .any(|f| matches!(f, AudioFileQueryFilter::ByArtistId(_)));

        // 检查是否有 starred 过滤器
        let starred_user_id = options.filters.iter().find_map(|f| {
            if let AudioFileQueryFilter::ByStarred(user_id) = f {
                Some(*user_id)
            } else {
                None
            }
        });

        // 检查是否需要 annotation join（非 starred 情况）
        let needs_annotation_for_order = matches!(options.order_by, AudioFileQueryOrderBy::ByPlayedCountDesc);

        // 构建 annotation JOIN（对于 ByStarred 需要在 JOIN 时就指定 user_id）
        let annotation_join = if let Some(user_id) = starred_user_id {
            values.push(user_id.into());
            param_index += 1;
            format!("JOIN annotation an ON af.id = an.item_id AND an.item_kind = 'audio_file' AND an.user_id = $1 AND an.starred = true")
        } else {
            "LEFT JOIN annotation an ON af.id = an.item_id AND an.item_kind = 'audio_file'".to_string()
        };

        // 构建 WHERE 条件（跳过 ByStarred，因为已在 JOIN 中处理）
        let mut where_parts = Vec::new();
        for filter in &options.filters {
            match filter {
                AudioFileQueryFilter::ById(id) => {
                    where_parts.push(format!("af.id = ${}", param_index));
                    values.push((*id).into());
                    param_index += 1;
                }
                AudioFileQueryFilter::ByArtistId(id) => {
                    where_parts.push(format!("p_filter.artist_id = ${}", param_index));
                    values.push((*id).into());
                    param_index += 1;
                }
                AudioFileQueryFilter::ByAlbumId(id) => {
                    where_parts.push(format!("af.album_id = ${}", param_index));
                    values.push((*id).into());
                    param_index += 1;
                }
                AudioFileQueryFilter::ByGenre(genre) => {
                    where_parts.push(format!(
                        "EXISTS (SELECT 1 FROM genre g WHERE g.id = af.genre_id AND lower(g.name) = lower(${}))",
                        param_index
                    ));
                    values.push(genre.clone().into());
                    param_index += 1;
                }
                AudioFileQueryFilter::ByYearRange(from_year, to_year) => {
                    where_parts.push(format!(
                        "af.year >= ${} AND af.year <= ${}",
                        param_index,
                        param_index + 1
                    ));
                    values.push((*from_year).into());
                    values.push((*to_year).into());
                    param_index += 2;
                }
                AudioFileQueryFilter::ByStarred(_) => {
                    // 已在 JOIN 条件中处理，跳过
                }
                AudioFileQueryFilter::All => {}
            }
        }

        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        // 构建额外的 JOIN
        let mut extra_joins = String::new();
        if needs_artist_filter {
            extra_joins.push_str(
                "\nJOIN participant p_filter ON af.id = p_filter.work_id AND p_filter.work_type = 'AudioFile'",
            );
        }

        // ORDER BY - DISTINCT ON (af.id) 要求首列为 af.id
        let outer_order_by = match options.order_by {
            AudioFileQueryOrderBy::ByTitle => "ORDER BY name",
            AudioFileQueryOrderBy::ByPlayedCountDesc => {
                "ORDER BY COALESCE(played_count, 0) DESC, name"
            }
            AudioFileQueryOrderBy::ByPlayedAtDesc => {
                "ORDER BY played_at DESC NULLS LAST, name"
            }
            AudioFileQueryOrderBy::Random => "ORDER BY random()",
        };

        // LIMIT & OFFSET
        let mut limit_offset = String::new();
        if let Some(limit) = options.limit {
            limit_offset.push_str(&format!(" LIMIT ${}", param_index));
            values.push(limit.into());
            param_index += 1;
        }
        if let Some(offset) = options.offset {
            limit_offset.push_str(&format!(" OFFSET ${}", param_index));
            values.push(offset.into());
        }

        let sql = format!(
            r#"SELECT * FROM (
                SELECT DISTINCT ON (af.id)
                    af.id, af.title as name, af.title as sort_name, af.title as order_name,
                    af.compilation, af.created_at as create_time, af.updated_at as update_time,
                    af.year, af.size, CAST(af.duration AS bigint) as duration, af.bit_rate, af.suffix,
                    (af.path_protocol || '://' || af.path_path) as path,
                    af.bpm, af.channels as channel_count, af.sample_rate, af.has_cover_art,
                    COALESCE(al.id, 0) as album_id, COALESCE(al.name, '') as album_name,
                    COALESCE(ar.id, 0) as artist_id, COALESCE(ar.name, '') as artist_name,
                    COALESCE(af.genre_id, 0) as genre_id, COALESCE(g.name, '') as genre_name,
                    an.played_count, an.played_at, an.rating, an.starred, an.starred_at
                FROM audio_file af
                LEFT JOIN album al ON af.album_id = al.id
                LEFT JOIN artist ar ON af.artist_id = ar.id
                LEFT JOIN genre g ON af.genre_id = g.id
                {annotation_join}{extra_joins}
                {where_clause}
                ORDER BY af.id
            ) AS sub
            {outer_order_by}
            {limit_offset}"#,
            annotation_join = annotation_join,
            extra_joins = extra_joins,
            where_clause = where_clause,
            outer_order_by = outer_order_by,
            limit_offset = limit_offset,
        );

        (sql, values)
    }

    /// 第二步：批量查询 participants
    async fn query_participants(
        &self,
        audio_file_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<Contributor>>, QueryError> {
        if audio_file_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: Vec<String> = (1..=audio_file_ids.len())
            .map(|i| format!("${}", i))
            .collect();
        let sql = format!(
            r#"SELECT p.work_id, p.artist_id, ar.name as artist_name, p.role, p.sub_role
               FROM participant p
               JOIN artist ar ON p.artist_id = ar.id
               WHERE p.work_id IN ({}) AND p.work_type = 'AudioFile'"#,
            placeholders.join(", ")
        );

        let values: Vec<Value> = audio_file_ids.iter().map(|id| (*id).into()).collect();
        let rows: Vec<ParticipantRow> =
            ParticipantRow::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        let mut result: HashMap<i64, Vec<Contributor>> = HashMap::new();
        for row in rows {
            result
                .entry(row.work_id)
                .or_default()
                .push(Contributor {
                    artist_id: row.artist_id,
                    artist_name: row.artist_name,
                    role: row.role,
                    sub_role: row.sub_role,
                });
        }
        Ok(result)
    }

    /// 第三步：批量查询副流派
    async fn query_secondary_genres(
        &self,
        audio_file_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<GenreSummary>>, QueryError> {
        if audio_file_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: Vec<String> = (1..=audio_file_ids.len())
            .map(|i| format!("${}", i))
            .collect();
        let sql = format!(
            r#"SELECT af.id as audio_file_id, g.id as genre_id, g.name as genre_name
               FROM audio_file af
               CROSS JOIN LATERAL unnest(af.genre_ids) AS genre_id_unnest
               JOIN genre g ON g.id = genre_id_unnest
               WHERE af.id IN ({})"#,
            placeholders.join(", ")
        );

        let values: Vec<Value> = audio_file_ids.iter().map(|id| (*id).into()).collect();
        let rows: Vec<SecondaryGenreRow> =
            SecondaryGenreRow::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        let mut result: HashMap<i64, Vec<GenreSummary>> = HashMap::new();
        for row in rows {
            let genres = result.entry(row.audio_file_id).or_default();
            // 去重
            if !genres.iter().any(|g| g.id == row.genre_id) {
                genres.push(GenreSummary {
                    id: row.genre_id,
                    name: row.genre_name,
                });
            }
        }
        Ok(result)
    }

    /// 组装最终结果
    fn assemble_audio_files(
        base_files: Vec<AudioFileBase>,
        participants: HashMap<i64, Vec<Contributor>>,
        secondary_genres: HashMap<i64, Vec<GenreSummary>>,
    ) -> Vec<AudioFile> {
        base_files
            .into_iter()
            .map(|base| {
                let contributors = participants.get(&base.id).cloned().unwrap_or_default();
                let genres = secondary_genres.get(&base.id).cloned().unwrap_or_default();

                AudioFile {
                    id: base.id,
                    library_id: 0,
                    path: base.path,
                    title: base.name.clone(),
                    album: base.album_name.clone(),
                    artists: Vec::new(),
                    album_artists: Vec::new(),
                    album_id: base.album_id,
                    has_cover_art: base.has_cover_art,
                    track_number: 0,
                    disc_number: 0,
                    disc_subtitle: String::new(),
                    year: base.year,
                    size: base.size,
                    suffix: base.suffix,
                    duration: base.duration,
                    bit_rate: base.bit_rate,
                    channels: base.channel_count.unwrap_or(0),
                    order_title: base.order_name.clone(),
                    bpm: base.bpm.unwrap_or(0),
                    name: base.name,
                    song_count: 1,
                    compilation: base.compilation,
                    sort_name: base.sort_name,
                    order_name: base.order_name,
                    annotation: Annotation {
                        play_count: base.played_count.unwrap_or(0),
                        play_date: base.played_at,
                        rating: base.rating.unwrap_or(0),
                        starred: base.starred.unwrap_or(false),
                        starred_at: base.starred_at,
                    },
                    genre: if base.genre_id > 0 {
                        Some(GenreSummary {
                            id: base.genre_id,
                            name: base.genre_name,
                        })
                    } else {
                        None
                    },
                    genres,
                    artist: ArtistSummary {
                        id: base.artist_id,
                        name: base.artist_name,
                    },
                    contributors,
                    created_at: base.create_time,
                    updated_at: base.update_time,
                }
            })
            .collect()
    }

    /// 执行完整的三步查询
    async fn query_audio_files(
        &self,
        options: AudioFileQueryOptions,
    ) -> Result<Vec<AudioFile>, QueryError> {
        // 第一步：查询基础数据
        let (sql, values) = Self::build_base_query_sql(&options);
        let base_files: Vec<AudioFileBase> =
            AudioFileBase::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        if base_files.is_empty() {
            return Ok(Vec::new());
        }

        // 收集所有 audio_file_id
        let ids: Vec<i64> = base_files.iter().map(|f| f.id).collect();

        // 第二步 & 第三步：并行查询 participants 和 secondary_genres
        let (participants, secondary_genres) = tokio::try_join!(
            self.query_participants(&ids),
            self.query_secondary_genres(&ids)
        )?;

        // 组装结果
        Ok(Self::assemble_audio_files(
            base_files,
            participants,
            secondary_genres,
        ))
    }
}

#[async_trait]
impl AudioFileDao for AudioFileDaoImpl {
    async fn get_by_id(&self, id: i64) -> Result<Option<AudioFile>, QueryError> {
        let options = AudioFileQueryOptions {
            filters: vec![AudioFileQueryFilter::ById(id)],
            ..Default::default()
        };
        let results = self.query_audio_files(options).await?;
        Ok(results.into_iter().next())
    }

    async fn get_by_album_id(&self, album_id: i64) -> Result<Vec<AudioFile>, QueryError> {
        let options = AudioFileQueryOptions {
            filters: vec![AudioFileQueryFilter::ByAlbumId(album_id)],
            ..Default::default()
        };
        self.query_audio_files(options).await
    }

    async fn get_by_artist_id(&self, artist_id: i64) -> Result<Vec<AudioFile>, QueryError> {
        let options = AudioFileQueryOptions {
            filters: vec![AudioFileQueryFilter::ByArtistId(artist_id)],
            ..Default::default()
        };
        self.query_audio_files(options).await
    }

    async fn get_all(&self) -> Result<Vec<AudioFile>, QueryError> {
        let options = AudioFileQueryOptions::default();
        self.query_audio_files(options).await
    }

    async fn get_top_songs_by_artist_id(
        &self,
        artist_id: i64,
        limit: i32,
    ) -> Result<Vec<AudioFile>, QueryError> {
        let options = AudioFileQueryOptions {
            filters: vec![AudioFileQueryFilter::ByArtistId(artist_id)],
            order_by: AudioFileQueryOrderBy::ByPlayedCountDesc,
            limit: Some(limit),
            offset: None,
        };
        self.query_audio_files(options).await
    }

    async fn get_random_songs(
        &self,
        genre: Option<&str>,
        from_year: Option<i32>,
        to_year: Option<i32>,
        limit: i32,
    ) -> Result<Vec<AudioFile>, QueryError> {
        let mut filters = Vec::new();
        if let Some(g) = genre {
            filters.push(AudioFileQueryFilter::ByGenre(g.to_string()));
        }
        if let (Some(from), Some(to)) = (from_year, to_year) {
            if from > 0 && to > 0 {
                filters.push(AudioFileQueryFilter::ByYearRange(from, to));
            }
        }

        let options = AudioFileQueryOptions {
            filters,
            order_by: AudioFileQueryOrderBy::Random,
            limit: Some(limit),
            offset: None,
        };
        self.query_audio_files(options).await
    }

    async fn get_by_genre(
        &self,
        genre: &str,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<AudioFile>, QueryError> {
        let options = AudioFileQueryOptions {
            filters: vec![AudioFileQueryFilter::ByGenre(genre.to_string())],
            order_by: AudioFileQueryOrderBy::ByTitle,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_audio_files(options).await
    }

    async fn get_by_starred(&self, user_id: i64) -> Result<Vec<AudioFile>, QueryError> {
        let options = AudioFileQueryOptions {
            filters: vec![AudioFileQueryFilter::ByStarred(user_id)],
            order_by: AudioFileQueryOrderBy::ByTitle,
            limit: None,
            offset: None,
        };
        self.query_audio_files(options).await
    }

    async fn get_most_played(&self, limit: i32) -> Result<Vec<AudioFile>, QueryError> {
        let options = AudioFileQueryOptions {
            filters: vec![],
            order_by: AudioFileQueryOrderBy::ByPlayedCountDesc,
            limit: Some(limit),
            offset: None,
        };
        self.query_audio_files(options).await
    }

    async fn get_recently_played(&self, limit: i32) -> Result<Vec<AudioFile>, QueryError> {
        let options = AudioFileQueryOptions {
            filters: vec![],
            order_by: AudioFileQueryOrderBy::ByPlayedAtDesc,
            limit: Some(limit),
            offset: None,
        };
        self.query_audio_files(options).await
    }

    async fn search(
        &self,
        query: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
        title: Option<&str>,
        newer_than: Option<i64>,
        offset: i32,
        limit: i32,
    ) -> Result<(Vec<AudioFile>, i64), QueryError> {
        // 构建搜索条件
        let mut where_parts = Vec::new();
        let mut values: Vec<Value> = Vec::new();
        let mut param_index = 1;

        if let Some(q) = query {
            if !q.is_empty() {
                where_parts.push(format!(
                    "(lower(af.title) LIKE lower(${}::text) OR lower(ar.name) LIKE lower(${}::text) OR lower(al.name) LIKE lower(${}::text))",
                    param_index, param_index, param_index
                ));
                values.push(format!("%{}%", q).into());
                param_index += 1;
            }
        }

        if let Some(a) = artist {
            if !a.is_empty() {
                where_parts.push(format!("lower(ar.name) LIKE lower(${}::text)", param_index));
                values.push(format!("%{}%", a).into());
                param_index += 1;
            }
        }

        if let Some(a) = album {
            if !a.is_empty() {
                where_parts.push(format!("lower(al.name) LIKE lower(${}::text)", param_index));
                values.push(format!("%{}%", a).into());
                param_index += 1;
            }
        }

        if let Some(t) = title {
            if !t.is_empty() {
                where_parts.push(format!("lower(af.title) LIKE lower(${}::text)", param_index));
                values.push(format!("%{}%", t).into());
                param_index += 1;
            }
        }

        if let Some(newer) = newer_than {
            if newer > 0 {
                where_parts.push(format!(
                    "af.created_at > to_timestamp(${}::bigint / 1000)",
                    param_index
                ));
                values.push(newer.into());
                param_index += 1;
            }
        }

        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        // 1. 查询总数
        let count_sql = format!(
            r#"SELECT COUNT(DISTINCT af.id) as total
               FROM audio_file af
               LEFT JOIN album al ON af.album_id = al.id
               LEFT JOIN artist ar ON af.artist_id = ar.id
               {}"#,
            where_clause
        );

        let count_result: Option<i64> = self
            .db
            .query_one(if values.is_empty() {
                Statement::from_string(DbBackend::Postgres, &count_sql)
            } else {
                Statement::from_sql_and_values(DbBackend::Postgres, &count_sql, values.clone())
            })
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?
            .map(|row| row.try_get_by_index::<i64>(0).unwrap_or(0));

        let total = count_result.unwrap_or(0);

        // 2. 查询基础数据（带分页）
        let mut query_values = values;
        query_values.push(limit.into());
        query_values.push(offset.into());

        let base_sql = format!(
            r#"SELECT * FROM (
                SELECT DISTINCT ON (af.id)
                    af.id, af.title as name, af.title as sort_name, af.title as order_name,
                    af.compilation, af.created_at as create_time, af.updated_at as update_time,
                    af.year, af.size, CAST(af.duration AS bigint) as duration, af.bit_rate, af.suffix,
                    (af.path_protocol || '://' || af.path_path) as path,
                    af.bpm, af.channels as channel_count, af.sample_rate, af.has_cover_art,
                    COALESCE(al.id, 0) as album_id, COALESCE(al.name, '') as album_name,
                    COALESCE(ar.id, 0) as artist_id, COALESCE(ar.name, '') as artist_name,
                    COALESCE(af.genre_id, 0) as genre_id, COALESCE(g.name, '') as genre_name,
                    an.played_count, an.played_at, an.rating, an.starred, an.starred_at
                FROM audio_file af
                LEFT JOIN album al ON af.album_id = al.id
                LEFT JOIN artist ar ON af.artist_id = ar.id
                LEFT JOIN genre g ON af.genre_id = g.id
                LEFT JOIN annotation an ON af.id = an.item_id AND an.item_kind = 'audio_file'
                {}
                ORDER BY af.id
            ) AS sub
            ORDER BY name
            LIMIT ${} OFFSET ${}"#,
            where_clause, param_index, param_index + 1
        );

        let base_files: Vec<AudioFileBase> =
            AudioFileBase::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &base_sql,
                query_values,
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        if base_files.is_empty() {
            return Ok((Vec::new(), total));
        }

        // 3. 批量查询关联数据
        let ids: Vec<i64> = base_files.iter().map(|f| f.id).collect();
        let (participants, secondary_genres) = tokio::try_join!(
            self.query_participants(&ids),
            self.query_secondary_genres(&ids)
        )?;

        // 4. 组装结果
        let audio_files = Self::assemble_audio_files(base_files, participants, secondary_genres);
        Ok((audio_files, total))
    }
}
