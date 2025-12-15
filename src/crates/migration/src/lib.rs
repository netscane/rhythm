pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_user_domain;
mod m20220101_000002_create_music_domain;
mod m20220101_000003_create_media_domain;
mod m20220101_000004_create_location_domain;
mod m20220101_000005_create_stats_domain;
mod m20220101_000006_create_activity_domain;
mod m20220101_000007_seed_data;
mod m20220101_000008_create_player_domain;
mod m20250202_000001_create_playlist_domain;
mod m20250203_000001_create_play_queue_domain;
mod m20250204_000001_create_transcoding_domain;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_user_domain::Migration),
            Box::new(m20220101_000002_create_music_domain::Migration),
            Box::new(m20220101_000003_create_media_domain::Migration),
            Box::new(m20220101_000004_create_location_domain::Migration),
            Box::new(m20220101_000005_create_stats_domain::Migration),
            Box::new(m20220101_000006_create_activity_domain::Migration),
            Box::new(m20220101_000007_seed_data::Migration),
            Box::new(m20220101_000008_create_player_domain::Migration),
            Box::new(m20250202_000001_create_playlist_domain::Migration),
            Box::new(m20250203_000001_create_play_queue_domain::Migration),
            Box::new(m20250204_000001_create_transcoding_domain::Migration),
        ]
    }
}
