use chrono::{DateTime, Local};
// for seed data
use infra::repository::postgres::command::db_data::{library, user};
use log::{info, warn};
use sea_orm::Set;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.seed_data(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove seed data
        let db = manager.get_connection();

        // Delete library first (due to foreign key constraints)
        library::Entity::delete_many()
            .filter(library::Column::Id.eq(1))
            .exec(db)
            .await?;

        // Delete user
        user::Entity::delete_many()
            .filter(user::Column::Id.eq(1))
            .exec(db)
            .await?;

        Ok(())
    }
}

impl Migration {
    async fn seed_data(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let zero_time = DateTime::from_timestamp(0, 0).unwrap().naive_utc();
        let now = Local::now().naive_utc();

        // Check if admin user already exists
        /*
        let existing_user = user::Entity::find_by_id(1).one(db).await?;
        if existing_user.is_none() {
            info!("Admin user not found, inserting seed admin user...");
            // Insert admin user
            let admin_user = user::ActiveModel {
                id: Set(1),
                username: Set("admin".to_owned()),
                name: Set("wafer".to_owned()),
                email: Set("test@gmail.com".to_owned()),
                is_admin: Set(true),
                password: Set("QvZZ2sWuTbuBXxvl".to_owned()),
                status: Set(1_i32), // UserStatus::Active
                last_login_at: Set(now),
                last_access_at: Set(now),
                last_op_time: Set(now),
                version: Set(1_i64),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };
            match admin_user.insert(db).await {
                Ok(_) => {
                    info!("Successfully inserted admin user with id: 1");
                }
                Err(e) => {
                    warn!("Failed to insert admin user: {}", e);
                    return Err(e);
                }
            }
        } else {
            info!("Admin user already exists, skipping insertion");
        }
        */

        // Check if test library already exists
        let existing_library = library::Entity::find_by_id(1).one(db).await?;
        if existing_library.is_none() {
            // Insert test library
            library::ActiveModel {
                id: Set(1),
                name: Set("nas".to_owned()),
                path_protocol: Set("local".to_owned()),
                //path_path: Set("/data/share/Music/G.E.M.邓紫棋/".to_owned()),
                path_path: Set(
                    //"/data/share/Music/2016 许镜清《西游记》主题音乐会 [WAV]/".to_owned()
                    "/data/share/Music/".to_owned(),
                ),
                scan_status: Set(1_i32),
                last_scan_at: Set(zero_time),
                version: Set(1_i64),
            }
            .insert(db)
            .await?;
        }

        Ok(())
    }
}
