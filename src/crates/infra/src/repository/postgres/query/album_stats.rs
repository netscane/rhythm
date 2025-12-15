use super::db_data::album_stats as stat_db;
use async_trait::async_trait;
use domain::value::AlbumId;
use model::album_stats::{AlbumStats, AlbumStatsAdjustment, AlbumStatsRepository};
use model::ModelError;
use sea_orm::*;

pub struct MysqlAlbumStatsRepository {
    db: DatabaseConnection,
}

impl MysqlAlbumStatsRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AlbumStatsRepository for MysqlAlbumStatsRepository {
    async fn adjust_stats(&self, adjustment: AlbumStatsAdjustment) -> Result<(), ModelError> {
        use sea_orm::sea_query::{Expr, Func, OnConflict};
        
        let album_id_i64: i64 = adjustment.album_id.clone().into();
        
        // For disk_numbers array handling, we need to:
        // 1. On INSERT: use the provided disk_number as initial array
        // 2. On UPDATE: append the disk_number if it doesn't exist (using PostgreSQL array_append)
        
        let initial_disk_numbers = if let Some(disk_num) = adjustment.disk_number {
            vec![disk_num]
        } else {
            vec![]
        };
        
        let initial_year = adjustment.year.unwrap_or(0);
        
        // Build the INSERT portion
        let active_model = stat_db::ActiveModel {
            album_id: Set(album_id_i64),
            duration: Set(adjustment.duration_delta),
            size: Set(adjustment.size_delta),
            song_count: Set(adjustment.song_count_delta),
            disk_numbers: Set(initial_disk_numbers.clone()),
            year: Set(initial_year),
        };
        
        // Build ON CONFLICT clause - use to_owned() to get ownership
        let mut on_conflict = OnConflict::column(stat_db::Column::AlbumId)
            .value(
                stat_db::Column::Duration,
                Expr::col((stat_db::Entity, stat_db::Column::Duration)).add(adjustment.duration_delta),
            )
            .value(
                stat_db::Column::Size,
                Expr::col((stat_db::Entity, stat_db::Column::Size)).add(adjustment.size_delta),
            )
            .value(
                stat_db::Column::SongCount,
                Expr::col((stat_db::Entity, stat_db::Column::SongCount)).add(adjustment.song_count_delta),
            )
            .to_owned();
        
        // Handle disk_numbers array update
        if let Some(disk_num) = adjustment.disk_number {
            // Use PostgreSQL-specific array operations
            // CASE WHEN disk_num = ANY(disk_numbers) THEN disk_numbers ELSE array_append(disk_numbers, disk_num) END
            on_conflict = on_conflict.value(
                stat_db::Column::DiskNumbers,
                Expr::cust_with_values(
                    "CASE WHEN $1 = ANY(disk_numbers) THEN disk_numbers ELSE array_append(disk_numbers, $1) END",
                    vec![disk_num]
                ),
            )
            .to_owned();
        }
        
        // Handle year update (only set if current is 0 or NULL)
        if let Some(year_val) = adjustment.year {
            on_conflict = on_conflict.value(
                stat_db::Column::Year,
                Expr::cust_with_values(
                    "CASE WHEN COALESCE(year, 0) = 0 THEN $1 ELSE year END",
                    vec![year_val]
                ),
            )
            .to_owned();
        }
        
        stat_db::Entity::insert(active_model)
            .on_conflict(on_conflict)
            .exec(&self.db)
            .await
            .map_err(|e| ModelError::ProjectionError(e.to_string()))?;
        
        Ok(())
    }

    async fn find_by_album_id(&self, album_id: AlbumId) -> Result<Option<AlbumStats>, ModelError> {
        let album_id_i64: i64 = album_id.into();
        let row = stat_db::Entity::find()
            .filter(stat_db::Column::AlbumId.eq(album_id_i64))
            .one(&self.db)
            .await
            .map_err(|e| ModelError::ProjectionError(e.to_string()))?;
        Ok(row.map(AlbumStats::from))
    }

    async fn save(&self, album_stats: AlbumStats) -> Result<(), ModelError> {
        let album_id_i64: i64 = album_stats.album_id.clone().into();

        // 先尝试更新，如果记录不存在则插入
        let update_result = stat_db::Entity::update_many()
            .filter(stat_db::Column::AlbumId.eq(album_id_i64))
            .set(stat_db::ActiveModel {
                album_id: Set(album_id_i64),
                duration: Set(album_stats.duration),
                size: Set(album_stats.size),
                song_count: Set(album_stats.song_count),
                disk_numbers: Set(album_stats.disk_numbers.clone()),
                year: Set(album_stats.year.unwrap_or(0)),
            })
            .exec(&self.db)
            .await;

        match update_result {
            Ok(update_result) if update_result.rows_affected > 0 => {
                // 成功更新
            }
            _ => {
                // 更新失败或没有记录被更新，尝试插入新记录
                let active_model = stat_db::ActiveModel {
                    album_id: Set(album_id_i64),
                    duration: Set(album_stats.duration),
                    size: Set(album_stats.size),
                    song_count: Set(album_stats.song_count),
                    disk_numbers: Set(album_stats.disk_numbers.clone()),
                    year: Set(album_stats.year.unwrap_or(0)),
                };

                // 使用 insert 方法，忽略 RecordNotFound 错误
                match active_model.insert(&self.db).await {
                    Ok(_) => {
                        // 成功插入
                    }
                    Err(sea_orm::DbErr::RecordNotFound(_)) => {
                        // 忽略 RecordNotFound 错误，记录可能已经存在
                    }
                    Err(e) => {
                        return Err(ModelError::ProjectionError(
                            "Failed to insert album stats".to_string() + &e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    async fn delete_by_album_id(&self, album_id: AlbumId) -> Result<(), ModelError> {
        let album_id_i64: i64 = album_id.into();
        stat_db::Entity::delete_many()
            .filter(stat_db::Column::AlbumId.eq(album_id_i64))
            .exec(&self.db)
            .await
            .map_err(|e| ModelError::ProjectionError(e.to_string()))?;
        Ok(())
    }
}
