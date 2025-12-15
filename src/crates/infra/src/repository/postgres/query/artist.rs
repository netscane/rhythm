use std::collections::HashMap;

use application::query::dao::ArtistDao;
use application::query::QueryError;
use async_trait::async_trait;
use model::artist::{Artist, ArtistInfo, ArtistStats};
use sea_orm::*;

pub struct ArtistDaoImpl {
    db: DatabaseConnection,
}

impl ArtistDaoImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

/// Artist 基础数据（不含 roles 一对多关系）
#[derive(Debug, Clone, FromQueryResult)]
struct ArtistBase {
    pub id: i64,
    pub name: String,
    pub sort_name: String,
    pub order_name: String,
    pub size: i64,
    pub album_count: i32,
    pub song_count: i32,
    pub duration: i64,
    pub played_count: Option<i32>,
    pub played_at: Option<chrono::NaiveDateTime>,
    pub rating: Option<i32>,
    pub starred: Option<bool>,
    pub starred_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

/// Role 统计数据
#[derive(Debug, Clone, FromQueryResult)]
struct RoleStatsRow {
    pub artist_id: i64,
    pub role: String,
    pub size: i64,
    pub album_count: i32,
    pub song_count: i32,
}

/// 查询过滤器
#[derive(Debug, Clone)]
enum ArtistQueryFilter {
    ById(i64),
    ByStarred(i64), // user_id
    All,
}

/// 排序方式
#[derive(Debug, Clone, Default)]
enum ArtistQueryOrderBy {
    #[default]
    BySortName,
    ByPlayedCountDesc,
    ByPlayedAtDesc,
    ByStarredAtDesc,
}

/// 查询选项
#[derive(Debug, Clone)]
struct ArtistQueryOptions {
    filter: ArtistQueryFilter,
    order_by: ArtistQueryOrderBy,
    limit: Option<i32>,
    offset: Option<i32>,
}

impl Default for ArtistQueryOptions {
    fn default() -> Self {
        Self {
            filter: ArtistQueryFilter::All,
            order_by: ArtistQueryOrderBy::BySortName,
            limit: None,
            offset: None,
        }
    }
}

impl ArtistDaoImpl {
    /// 第一步：查询 artist 基础信息（只返回有 'Artist' role 的艺术家）
    fn build_base_query_sql(options: &ArtistQueryOptions) -> (String, Vec<Value>) {
        let mut values: Vec<Value> = Vec::new();
        let mut param_index = 1;

        // 构建 annotation JOIN 条件（对于 ByStarred 需要在 JOIN 时就指定 user_id）
        let annotation_join = match &options.filter {
            ArtistQueryFilter::ByStarred(user_id) => {
                values.push((*user_id).into());
                param_index += 1;
                format!("JOIN annotation an ON ar.id = an.item_id AND an.item_kind = 'artist' AND an.user_id = $1 AND an.starred = true")
            }
            _ => "LEFT JOIN annotation an ON ar.id = an.item_id AND an.item_kind = 'artist'".to_string(),
        };

        // 构建 WHERE 条件
        let where_clause = match &options.filter {
            ArtistQueryFilter::ById(id) => {
                values.push((*id).into());
                param_index += 1;
                format!("WHERE ar.id = ${} AND ps.role = 'Artist'", param_index - 1)
            }
            ArtistQueryFilter::ByStarred(_) => {
                // user_id 已在 JOIN 条件中使用
                "WHERE ps.role = 'Artist'".to_string()
            }
            ArtistQueryFilter::All => "WHERE ps.role = 'Artist'".to_string(),
        };

        // ORDER BY - DISTINCT ON (ar.id) 要求首列为 ar.id
        let outer_order_by = match &options.order_by {
            ArtistQueryOrderBy::BySortName => "ORDER BY sort_name",
            ArtistQueryOrderBy::ByPlayedCountDesc => "ORDER BY COALESCE(played_count, 0) DESC, sort_name",
            ArtistQueryOrderBy::ByPlayedAtDesc => "ORDER BY played_at DESC NULLS LAST, sort_name",
            ArtistQueryOrderBy::ByStarredAtDesc => "ORDER BY starred_at DESC NULLS LAST, sort_name",
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
                SELECT DISTINCT ON (ar.id)
                    ar.id, ar.name, ar.sort_name, ar.sort_name as order_name,
                    ps.size, ps.album_count, ps.song_count, ps.duration,
                    an.played_count, an.played_at, an.rating, an.starred, an.starred_at,
                    ar.update_time as updated_at
                FROM artist ar
                JOIN participant_stats ps ON ar.id = ps.artist_id
                {annotation_join}
                {where_clause}
                ORDER BY ar.id
            ) AS sub
            {outer_order_by}
            {limit_offset}"#,
            annotation_join = annotation_join,
            where_clause = where_clause,
            outer_order_by = outer_order_by,
            limit_offset = limit_offset,
        );

        (sql, values)
    }

    /// 第二步：批量查询所有 role 统计数据
    async fn query_role_stats(
        &self,
        artist_ids: &[i64],
    ) -> Result<HashMap<i64, HashMap<String, ArtistStats>>, QueryError> {
        if artist_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: Vec<String> = (1..=artist_ids.len())
            .map(|i| format!("${}", i))
            .collect();
        let sql = format!(
            r#"SELECT artist_id, role, size, album_count, song_count
               FROM participant_stats
               WHERE artist_id IN ({})"#,
            placeholders.join(", ")
        );

        let values: Vec<Value> = artist_ids.iter().map(|id| (*id).into()).collect();
        let rows: Vec<RoleStatsRow> =
            RoleStatsRow::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        let mut result: HashMap<i64, HashMap<String, ArtistStats>> = HashMap::new();
        for row in rows {
            result
                .entry(row.artist_id)
                .or_default()
                .insert(
                    row.role,
                    ArtistStats {
                        size: row.size,
                        album_count: row.album_count,
                        song_count: row.song_count,
                    },
                );
        }
        Ok(result)
    }

    /// 组装最终结果
    fn assemble_artists(
        base_artists: Vec<ArtistBase>,
        role_stats: HashMap<i64, HashMap<String, ArtistStats>>,
    ) -> Vec<Artist> {
        base_artists
            .into_iter()
            .map(|base| {
                let roles = role_stats.get(&base.id).cloned().unwrap_or_default();

                Artist {
                    id: base.id,
                    name: base.name,
                    sort_name: base.sort_name,
                    order_name: base.order_name,
                    size: base.size,
                    album_count: base.album_count,
                    song_count: base.song_count,
                    played_count: base.played_count.unwrap_or(0),
                    played_at: base.played_at,
                    rating: base.rating.unwrap_or(0),
                    starred: base.starred.unwrap_or(false),
                    starred_at: base.starred_at,
                    updated_at: base.updated_at,
                    mbz_artist_id: None,
                    roles,
                }
            })
            .collect()
    }

    /// 执行完整的两步查询
    async fn query_artists(
        &self,
        options: ArtistQueryOptions,
    ) -> Result<Vec<Artist>, QueryError> {
        // 第一步：查询基础数据
        let (sql, values) = Self::build_base_query_sql(&options);
        let base_artists: Vec<ArtistBase> =
            ArtistBase::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                values,
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        if base_artists.is_empty() {
            return Ok(Vec::new());
        }

        // 收集所有 artist_id
        let ids: Vec<i64> = base_artists.iter().map(|a| a.id).collect();

        // 第二步：查询 role 统计数据
        let role_stats = self.query_role_stats(&ids).await?;

        // 组装结果
        Ok(Self::assemble_artists(base_artists, role_stats))
    }
}

#[async_trait]
impl ArtistDao for ArtistDaoImpl {
    async fn get_by_id(&self, id: i64) -> Result<Option<Artist>, QueryError> {
        let options = ArtistQueryOptions {
            filter: ArtistQueryFilter::ById(id),
            ..Default::default()
        };
        let results = self.query_artists(options).await?;
        Ok(results.into_iter().next())
    }

    async fn get_all(&self) -> Result<Vec<Artist>, QueryError> {
        let options = ArtistQueryOptions::default();
        self.query_artists(options).await
    }

    async fn get_by_sort_name(&self, artist_name: &str) -> Result<Option<Artist>, QueryError> {
        // 第一步：查询匹配的 artist 基础信息
        // 使用子查询解决 DISTINCT ON 与 ORDER BY 冲突
        let sql = r#"SELECT * FROM (
                SELECT DISTINCT ON (ar.id)
                    ar.id, ar.name, ar.sort_name, ar.sort_name as order_name,
                    ps.size, ps.album_count, ps.song_count, ps.duration,
                    an.played_count, an.played_at, an.rating, an.starred, an.starred_at,
                    ar.update_time as updated_at
                FROM artist ar
                JOIN participant_stats ps ON ar.id = ps.artist_id
                LEFT JOIN annotation an ON ar.id = an.item_id AND an.item_kind = 'artist'
                WHERE ps.role = 'Artist' AND (lower(ar.sort_name) = lower($1) OR lower(ar.name) = lower($1))
                ORDER BY ar.id
            ) AS sub
            ORDER BY CASE WHEN lower(sort_name) = lower($1) THEN 0 ELSE 1 END
            LIMIT 1"#;

        let base_artists: Vec<ArtistBase> =
            ArtistBase::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                vec![artist_name.into()],
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        if base_artists.is_empty() {
            return Ok(None);
        }

        // 第二步：查询 role 统计数据
        let ids: Vec<i64> = base_artists.iter().map(|a| a.id).collect();
        let role_stats = self.query_role_stats(&ids).await?;

        // 组装结果
        Ok(Self::assemble_artists(base_artists, role_stats).into_iter().next())
    }

    async fn get_artist_info(&self, artist_id: i64) -> Result<Option<ArtistInfo>, QueryError> {
        let sql = r#"
            SELECT ar.id, ar.name, null as biography
            FROM artist ar
            WHERE ar.id = $1
        "#;
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, vec![artist_id.into()]);
        let result = self
            .db
            .query_one(stmt)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        if let Some(row) = result {
            Ok(Some(ArtistInfo {
                id: row
                    .try_get::<i64>("", "id")
                    .map_err(|e| QueryError::DbError(e.to_string()))?,
                name: row
                    .try_get::<String>("", "name")
                    .map_err(|e| QueryError::DbError(e.to_string()))?,
                biography: row
                    .try_get::<Option<String>>("", "biography")
                    .map_err(|e| QueryError::DbError(e.to_string()))?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_random_artist_id(
        &self,
        exclude_artist_id: Option<i64>,
    ) -> Result<Option<i64>, QueryError> {
        let sql = if let Some(_exclude_id) = exclude_artist_id {
            r#"SELECT DISTINCT ar.id 
                FROM artist ar 
                JOIN participant_stats ps ON ar.id = ps.artist_id
                WHERE ps.role = 'Artist' AND ar.id != $1
                ORDER BY random()
                LIMIT 1"#
        } else {
            r#"SELECT DISTINCT ar.id 
                FROM artist ar 
                JOIN participant_stats ps ON ar.id = ps.artist_id
                WHERE ps.role = 'Artist'
                ORDER BY random()
                LIMIT 1"#
        };

        let random_id: Option<i64> = if let Some(exclude_id) = exclude_artist_id {
            let stmt =
                Statement::from_sql_and_values(DbBackend::Postgres, sql, vec![exclude_id.into()]);
            let result = self
                .db
                .query_one(stmt)
                .await
                .map_err(|e| QueryError::DbError(e.to_string()))?;
            result.and_then(|row| row.try_get::<i64>("", "id").ok())
        } else {
            let stmt = Statement::from_string(DbBackend::Postgres, sql);
            let result = self
                .db
                .query_one(stmt)
                .await
                .map_err(|e| QueryError::DbError(e.to_string()))?;
            result.and_then(|row| row.try_get::<i64>("", "id").ok())
        };

        Ok(random_id)
    }

    async fn get_by_starred(&self, user_id: i64) -> Result<Vec<Artist>, QueryError> {
        let options = ArtistQueryOptions {
            filter: ArtistQueryFilter::ByStarred(user_id),
            order_by: ArtistQueryOrderBy::ByStarredAtDesc,
            ..Default::default()
        };
        self.query_artists(options).await
    }

    async fn get_most_played(&self, limit: i32) -> Result<Vec<Artist>, QueryError> {
        let options = ArtistQueryOptions {
            filter: ArtistQueryFilter::All,
            order_by: ArtistQueryOrderBy::ByPlayedCountDesc,
            limit: Some(limit),
            offset: None,
        };
        self.query_artists(options).await
    }

    async fn get_recently_played(&self, limit: i32) -> Result<Vec<Artist>, QueryError> {
        let options = ArtistQueryOptions {
            filter: ArtistQueryFilter::All,
            order_by: ArtistQueryOrderBy::ByPlayedAtDesc,
            limit: Some(limit),
            offset: None,
        };
        self.query_artists(options).await
    }

    async fn search(
        &self,
        query: &str,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<Artist>, QueryError> {
        // 第一步：搜索匹配的 artist 基础信息
        // 使用子查询解决 DISTINCT ON 与 ORDER BY 冲突
        let sql = r#"SELECT * FROM (
                SELECT DISTINCT ON (ar.id)
                    ar.id, ar.name, ar.sort_name, ar.sort_name as order_name,
                    ps.size, ps.album_count, ps.song_count, ps.duration,
                    an.played_count, an.played_at, an.rating, an.starred, an.starred_at,
                    ar.update_time as updated_at
                FROM artist ar
                JOIN participant_stats ps ON ar.id = ps.artist_id
                LEFT JOIN annotation an ON ar.id = an.item_id AND an.item_kind = 'artist'
                WHERE ps.role = 'Artist' AND (lower(ar.name) LIKE lower($1) OR lower(ar.sort_name) LIKE lower($1))
                ORDER BY ar.id
            ) AS sub
            ORDER BY sort_name
            LIMIT $2 OFFSET $3"#;

        let search_pattern = format!("%{}%", query);
        let base_artists: Vec<ArtistBase> =
            ArtistBase::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                vec![search_pattern.into(), limit.into(), offset.into()],
            ))
            .all(&self.db)
            .await
            .map_err(|e| QueryError::DbError(e.to_string()))?;

        if base_artists.is_empty() {
            return Ok(Vec::new());
        }

        // 第二步：查询 role 统计数据
        let ids: Vec<i64> = base_artists.iter().map(|a| a.id).collect();
        let role_stats = self.query_role_stats(&ids).await?;

        // 组装结果
        Ok(Self::assemble_artists(base_artists, role_stats))
    }
}
