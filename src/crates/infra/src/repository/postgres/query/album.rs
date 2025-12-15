use std::collections::HashMap;

use application::query::dao::AlbumDao;
use application::query::QueryError;
use async_trait::async_trait;
use model::album::{Album, AlbumInfo, Discs};
use model::shared::{Annotation, ArtistSummary, Contributor, GenreSummary};
use sea_orm::*;

pub struct AlbumDaoImpl {
    db: DatabaseConnection,
}

impl AlbumDaoImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

/// Album 基础数据（不含 contributors 和 genres 一对多关系）
#[derive(Debug, Clone, FromQueryResult)]
struct AlbumBase {
    pub id: i64,
    pub name: String,
    pub sort_name: String,
    pub order_name: String,
    pub compilation: bool,
    pub create_time: chrono::NaiveDateTime,
    pub update_time: chrono::NaiveDateTime,
    pub artist_id: i64,
    pub artist_name: String,
    pub size: i64,
    pub song_count: i32,
    pub duration: i64,
    pub disk_numbers: Vec<i32>,
    pub year: i32,
    pub played_count: Option<i32>,
    pub played_at: Option<chrono::NaiveDateTime>,
    pub rating: Option<i32>,
    pub starred: Option<bool>,
    pub starred_at: Option<chrono::NaiveDateTime>,
    pub genre_id: i64,
    pub genre_name: String,
}

/// Contributor 数据
#[derive(Debug, Clone, FromQueryResult)]
struct ContributorRow {
    pub album_id: i64,
    pub artist_id: i64,
    pub artist_name: String,
    pub role: String,
    pub sub_role: Option<String>,
}

/// 副流派数据
#[derive(Debug, Clone, FromQueryResult)]
struct SecondaryGenreRow {
    pub album_id: i64,
    pub genre_id: i64,
    pub genre_name: String,
}

/// 查询过滤器
#[derive(Debug, Clone)]
enum AlbumQueryFilter {
    ById(i64),
    ByArtistId(i64),
    ByStarred(i64), // user_id
    ByGenre(String),
    ByYearRange(i32, i32),
    All,
}

/// 排序方式
#[derive(Debug, Clone, Default)]
enum AlbumQueryOrderBy {
    #[default]
    ByName,
    ByNewest,
    ByRecent,
    ByRandom,
    ByArtist,
    ByFrequent,
    ByStarred,
    ByRating,
    ByYear,
}

/// 查询选项
#[derive(Debug, Clone)]
struct AlbumQueryOptions {
    filter: AlbumQueryFilter,
    order_by: AlbumQueryOrderBy,
    limit: Option<i32>,
    offset: Option<i32>,
}

impl Default for AlbumQueryOptions {
    fn default() -> Self {
        Self {
            filter: AlbumQueryFilter::All,
            order_by: AlbumQueryOrderBy::ByName,
            limit: None,
            offset: None,
        }
    }
}

impl AlbumDaoImpl {
    /// 第一步：构建基础查询 SQL
    fn build_base_query_sql(options: &AlbumQueryOptions) -> (String, Vec<Value>) {
        let mut values: Vec<Value> = Vec::new();
        let mut param_index = 1;

        // 检查是否需要 artist filter join
        let needs_artist_filter = matches!(options.filter, AlbumQueryFilter::ByArtistId(_));

        // 构建 annotation JOIN 条件（对于 ByStarred 需要在 JOIN 时就指定 user_id）
        let annotation_join = match &options.filter {
            AlbumQueryFilter::ByStarred(user_id) => {
                values.push((*user_id).into());
                param_index += 1;
                format!("JOIN annotation an ON al.id = an.item_id AND an.item_kind = 'album' AND an.user_id = $1 AND an.starred = true")
            }
            _ => "LEFT JOIN annotation an ON al.id = an.item_id AND an.item_kind = 'album'".to_string(),
        };

        // 构建 WHERE 条件
        let where_clause = match &options.filter {
            AlbumQueryFilter::ById(id) => {
                values.push((*id).into());
                param_index += 1;
                format!("WHERE al.id = ${}", param_index - 1)
            }
            AlbumQueryFilter::ByArtistId(id) => {
                values.push((*id).into());
                param_index += 1;
                format!("WHERE p_filter.artist_id = ${}", param_index - 1)
            }
            AlbumQueryFilter::ByStarred(_) => {
                // user_id 已在 JOIN 条件中使用
                String::new()
            }
            AlbumQueryFilter::ByGenre(genre) => {
                values.push(genre.clone().into());
                param_index += 1;
                format!("WHERE lower(g.name) = lower(${})", param_index - 1)
            }
            AlbumQueryFilter::ByYearRange(from, to) => {
                values.push((*from).into());
                values.push((*to).into());
                param_index += 2;
                format!("WHERE als.year >= ${} AND als.year <= ${}", param_index - 2, param_index - 1)
            }
            AlbumQueryFilter::All => String::new(),
        };

        // 额外的 JOIN
        let extra_joins = if needs_artist_filter {
            "\nJOIN participant p_filter ON al.id = p_filter.work_id AND p_filter.work_type = 'Album'"
        } else {
            ""
        };

        // ORDER BY - DISTINCT ON (al.id) 要求 ORDER BY 首列必须是 al.id
        // 内层查询只用 al.id 排序以满足 DISTINCT ON 要求
        let inner_order_by = "ORDER BY al.id";
        
        // 外层排序
        let outer_order_by = match options.order_by {
            AlbumQueryOrderBy::ByName => "ORDER BY sort_name",
            AlbumQueryOrderBy::ByNewest => "ORDER BY create_time DESC",
            AlbumQueryOrderBy::ByRecent => "ORDER BY played_at DESC NULLS LAST",
            AlbumQueryOrderBy::ByRandom => "ORDER BY random()",
            AlbumQueryOrderBy::ByArtist => "ORDER BY artist_name, sort_name",
            AlbumQueryOrderBy::ByFrequent => "ORDER BY COALESCE(played_count, 0) DESC",
            AlbumQueryOrderBy::ByStarred => "ORDER BY starred_at DESC NULLS LAST",
            AlbumQueryOrderBy::ByRating => "ORDER BY COALESCE(rating, 0) DESC",
            AlbumQueryOrderBy::ByYear => "ORDER BY year, sort_name",
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
                SELECT DISTINCT ON (al.id)
                    al.id, al.name, al.sort_name, al.sort_name as order_name,
                    al.compilation, al.create_time, al.update_time,
                    COALESCE(ar.id, 0) as artist_id, COALESCE(ar.name, '') as artist_name,
                    als.size, als.song_count, als.duration, als.disk_numbers, als.year,
                    an.played_count, an.played_at, an.rating, an.starred, an.starred_at,
                    COALESCE(al.genre_id, 0) as genre_id, COALESCE(g.name, '') as genre_name
                FROM album al
                JOIN album_stats als ON al.id = als.album_id
                {annotation_join}
                LEFT JOIN artist ar ON al.artist_id = ar.id
                LEFT JOIN genre g ON al.genre_id = g.id{extra_joins}
                {where_clause}
                {inner_order_by}
            ) AS sub
            {outer_order_by}
            {limit_offset}"#,
            annotation_join = annotation_join,
            extra_joins = extra_joins,
            where_clause = where_clause,
            inner_order_by = inner_order_by,
            outer_order_by = outer_order_by,
            limit_offset = limit_offset,
        );

        (sql, values)
    }

    /// 第二步：批量查询 contributors
    async fn query_contributors(
        &self,
        album_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<Contributor>>, QueryError> {
        if album_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: Vec<String> = (1..=album_ids.len())
            .map(|i| format!("${}", i))
            .collect();
        let sql = format!(
            r#"SELECT p.work_id as album_id, p.artist_id, ar.name as artist_name, p.role, p.sub_role
               FROM participant p
               JOIN artist ar ON p.artist_id = ar.id
               WHERE p.work_id IN ({}) AND p.work_type = 'Album'"#,
            placeholders.join(", ")
        );

        let values: Vec<Value> = album_ids.iter().map(|id| (*id).into()).collect();
        let rows: Vec<ContributorRow> =
            ContributorRow::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        let mut result: HashMap<i64, Vec<Contributor>> = HashMap::new();
        for row in rows {
            let contributors = result.entry(row.album_id).or_default();
            // 去重
            if !contributors.iter().any(|c| c.artist_id == row.artist_id && c.role == row.role) {
                contributors.push(Contributor {
                    artist_id: row.artist_id,
                    artist_name: row.artist_name,
                    role: row.role,
                    sub_role: row.sub_role,
                });
            }
        }
        Ok(result)
    }

    /// 第三步：批量查询副流派
    async fn query_secondary_genres(
        &self,
        album_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<GenreSummary>>, QueryError> {
        if album_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: Vec<String> = (1..=album_ids.len())
            .map(|i| format!("${}", i))
            .collect();
        let sql = format!(
            r#"SELECT al.id as album_id, g.id as genre_id, g.name as genre_name
               FROM album al
               CROSS JOIN LATERAL unnest(al.genre_ids) AS genre_id_unnest
               JOIN genre g ON g.id = genre_id_unnest
               WHERE al.id IN ({})"#,
            placeholders.join(", ")
        );

        let values: Vec<Value> = album_ids.iter().map(|id| (*id).into()).collect();
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
            let genres = result.entry(row.album_id).or_default();
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
    fn assemble_albums(
        base_albums: Vec<AlbumBase>,
        contributors: HashMap<i64, Vec<Contributor>>,
        secondary_genres: HashMap<i64, Vec<GenreSummary>>,
    ) -> Vec<Album> {
        base_albums
            .into_iter()
            .map(|base| {
                let album_contributors = contributors.get(&base.id).cloned().unwrap_or_default();
                let genres = secondary_genres.get(&base.id).cloned().unwrap_or_default();

                // 构建 disc_numbers 到 Discs
                let mut discs: Discs = HashMap::new();
                for disk_num in base.disk_numbers.iter() {
                    if *disk_num == 0 {
                        discs.insert(0, "#".to_string());
                    } else {
                        discs.insert(*disk_num, format!("Disc {}", *disk_num));
                    }
                }

                Album {
                    id: base.id,
                    library_id: 0,
                    name: base.name,
                    song_count: base.song_count,
                    duration: base.duration,
                    year: if base.year != 0 { Some(base.year) } else { None },
                    compilation: base.compilation,
                    size: base.size,
                    discs,
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
                    contributors: album_contributors,
                    created_at: base.create_time,
                    updated_at: base.update_time,
                }
            })
            .collect()
    }

    /// 执行完整的三步查询
    async fn query_albums(&self, options: AlbumQueryOptions) -> Result<Vec<Album>, QueryError> {
        // 第一步：查询基础数据
        let (sql, values) = Self::build_base_query_sql(&options);
        let base_albums: Vec<AlbumBase> =
            AlbumBase::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        if base_albums.is_empty() {
            return Ok(Vec::new());
        }

        // 收集所有 album_id
        let ids: Vec<i64> = base_albums.iter().map(|a| a.id).collect();

        // 第二步 & 第三步：并行查询 contributors 和 secondary_genres
        let (contributors, secondary_genres) = tokio::try_join!(
            self.query_contributors(&ids),
            self.query_secondary_genres(&ids)
        )?;

        // 组装结果
        Ok(Self::assemble_albums(base_albums, contributors, secondary_genres))
    }

    /// 带总数的查询（用于分页）
    async fn query_albums_with_count(
        &self,
        options: AlbumQueryOptions,
    ) -> Result<(Vec<Album>, i64), QueryError> {
        // 构建 count SQL
        let count_where = match &options.filter {
            AlbumQueryFilter::ById(id) => {
                format!("WHERE al.id = {}", id)
            }
            AlbumQueryFilter::ByArtistId(id) => {
                format!(
                    "WHERE EXISTS (SELECT 1 FROM participant p WHERE p.work_id = al.id AND p.work_type = 'Album' AND p.artist_id = {})",
                    id
                )
            }
            AlbumQueryFilter::ByStarred(user_id) => {
                format!("WHERE an.starred = true AND an.user_id = {}", user_id)
            }
            AlbumQueryFilter::ByGenre(genre) => {
                format!("WHERE lower(g.name) = lower('{}')", genre.replace('\'', "''"))
            }
            AlbumQueryFilter::ByYearRange(from, to) => {
                format!("WHERE als.year >= {} AND als.year <= {}", from, to)
            }
            AlbumQueryFilter::All => String::new(),
        };

        let count_sql = format!(
            r#"SELECT COUNT(DISTINCT al.id) as total
               FROM album al
               JOIN album_stats als ON al.id = als.album_id
               LEFT JOIN annotation an ON al.id = an.item_id AND an.item_kind = 'album'
               LEFT JOIN genre g ON al.genre_id = g.id
               {}"#,
            count_where
        );

        let count_result: Option<i64> = self
            .db
            .query_one(Statement::from_string(DbBackend::Postgres, &count_sql))
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?
            .map(|row| row.try_get_by_index::<i64>(0).unwrap_or(0));

        let total = count_result.unwrap_or(0);

        // 查询数据
        let albums = self.query_albums(options).await?;

        Ok((albums, total))
    }
}

#[async_trait]
impl AlbumDao for AlbumDaoImpl {
    async fn get_by_id(&self, id: i64) -> Result<Option<Album>, QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::ById(id),
            ..Default::default()
        };
        let results = self.query_albums(options).await?;
        Ok(results.into_iter().next())
    }

    async fn get_by_artist_id(&self, artist_id: i64) -> Result<Vec<Album>, QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::ByArtistId(artist_id),
            ..Default::default()
        };
        self.query_albums(options).await
    }

    async fn get_all(&self) -> Result<Vec<Album>, QueryError> {
        let options = AlbumQueryOptions::default();
        self.query_albums(options).await
    }

    async fn get_album_info(&self, album_id: i64) -> Result<Option<AlbumInfo>, QueryError> {
        let sql = r#"
            SELECT al.id, al.description 
            FROM album al
            WHERE al.id = $1
        "#;
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, vec![album_id.into()]);
        let result = self
            .db
            .query_one(stmt)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        if let Some(row) = result {
            Ok(Some(AlbumInfo {
                id: row
                    .try_get::<i64>("", "id")
                    .map_err(|e| QueryError::DbError(e.to_string()))?,
                description: row
                    .try_get::<Option<String>>("", "description")
                    .map_err(|e| QueryError::DbError(e.to_string()))?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_by_newest(&self, offset: i32, limit: i32) -> Result<(Vec<Album>, i64), QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::All,
            order_by: AlbumQueryOrderBy::ByNewest,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_albums_with_count(options).await
    }

    async fn get_by_recent(&self, offset: i32, limit: i32) -> Result<(Vec<Album>, i64), QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::All,
            order_by: AlbumQueryOrderBy::ByRecent,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_albums_with_count(options).await
    }

    async fn get_by_random(&self, offset: i32, limit: i32) -> Result<(Vec<Album>, i64), QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::All,
            order_by: AlbumQueryOrderBy::ByRandom,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_albums_with_count(options).await
    }

    async fn get_by_name(&self, offset: i32, limit: i32) -> Result<(Vec<Album>, i64), QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::All,
            order_by: AlbumQueryOrderBy::ByName,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_albums_with_count(options).await
    }

    async fn get_by_artist(&self, offset: i32, limit: i32) -> Result<(Vec<Album>, i64), QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::All,
            order_by: AlbumQueryOrderBy::ByArtist,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_albums_with_count(options).await
    }

    async fn get_by_frequent(
        &self,
        offset: i32,
        limit: i32,
    ) -> Result<(Vec<Album>, i64), QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::All,
            order_by: AlbumQueryOrderBy::ByFrequent,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_albums_with_count(options).await
    }

    async fn get_by_starred(
        &self,
        user_id: i64,
        offset: i32,
        limit: i32,
    ) -> Result<(Vec<Album>, i64), QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::ByStarred(user_id),
            order_by: AlbumQueryOrderBy::ByStarred,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_albums_with_count(options).await
    }

    async fn get_starred(&self, user_id: i64) -> Result<Vec<Album>, QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::ByStarred(user_id),
            order_by: AlbumQueryOrderBy::ByStarred,
            limit: None,
            offset: None,
        };
        self.query_albums(options).await
    }

    async fn get_by_rating(&self, offset: i32, limit: i32) -> Result<(Vec<Album>, i64), QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::All,
            order_by: AlbumQueryOrderBy::ByRating,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_albums_with_count(options).await
    }

    async fn get_by_genre(
        &self,
        genre: &str,
        offset: i32,
        limit: i32,
    ) -> Result<(Vec<Album>, i64), QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::ByGenre(genre.to_string()),
            order_by: AlbumQueryOrderBy::ByName,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_albums_with_count(options).await
    }

    async fn get_by_year(
        &self,
        from_year: i32,
        to_year: i32,
        offset: i32,
        limit: i32,
    ) -> Result<(Vec<Album>, i64), QueryError> {
        let options = AlbumQueryOptions {
            filter: AlbumQueryFilter::ByYearRange(from_year, to_year),
            order_by: AlbumQueryOrderBy::ByYear,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.query_albums_with_count(options).await
    }

    async fn search(
        &self,
        query: &str,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<Album>, QueryError> {
        // 第一步：搜索匹配的 album 基础信息
        // 使用子查询解决 DISTINCT ON 与 ORDER BY 冲突
        let sql = r#"SELECT * FROM (
                SELECT DISTINCT ON (al.id)
                    al.id, al.name, al.sort_name, al.sort_name as order_name,
                    al.compilation, al.create_time, al.update_time,
                    COALESCE(ar.id, 0) as artist_id, COALESCE(ar.name, '') as artist_name,
                    als.size, als.song_count, als.duration, als.disk_numbers, als.year,
                    an.played_count, an.played_at, an.rating, an.starred, an.starred_at,
                    COALESCE(al.genre_id, 0) as genre_id, COALESCE(g.name, '') as genre_name
                FROM album al
                JOIN album_stats als ON al.id = als.album_id
                LEFT JOIN annotation an ON al.id = an.item_id AND an.item_kind = 'album'
                LEFT JOIN artist ar ON al.artist_id = ar.id
                LEFT JOIN genre g ON al.genre_id = g.id
                WHERE lower(al.name) LIKE lower($1) OR lower(al.sort_name) LIKE lower($1)
                ORDER BY al.id
            ) AS sub
            ORDER BY sort_name
            LIMIT $2 OFFSET $3"#;

        let search_pattern = format!("%{}%", query);
        let base_albums: Vec<AlbumBase> =
            AlbumBase::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                vec![search_pattern.into(), limit.into(), offset.into()],
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        if base_albums.is_empty() {
            return Ok(Vec::new());
        }

        // 收集所有 album_id
        let ids: Vec<i64> = base_albums.iter().map(|a| a.id).collect();

        // 第二步 & 第三步：并行查询 contributors 和 secondary_genres
        let (contributors, secondary_genres) = tokio::try_join!(
            self.query_contributors(&ids),
            self.query_secondary_genres(&ids)
        )?;

        // 组装结果
        Ok(Self::assemble_albums(base_albums, contributors, secondary_genres))
    }
}
