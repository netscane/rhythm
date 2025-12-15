use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create playback_history table
        manager
            .create_table(
                Table::create()
                    .table(PlaybackHistory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlaybackHistory::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(
                        ColumnDef::new(PlaybackHistory::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlaybackHistory::AudioFileId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlaybackHistory::ScrobbledAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for playback_history table
        manager
            .create_index(
                Index::create()
                    .name("idx_playback_history_user_id")
                    .table(PlaybackHistory::Table)
                    .col(PlaybackHistory::UserId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_playback_history_user_scrobbled")
                    .table(PlaybackHistory::Table)
                    .col(PlaybackHistory::UserId)
                    .col(PlaybackHistory::ScrobbledAt)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create annotation table
        manager
            .create_table(
                Table::create()
                    .table(Annotation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Annotation::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Annotation::UserId).big_integer().not_null())
                    .col(ColumnDef::new(Annotation::ItemKind).string().not_null())
                    .col(ColumnDef::new(Annotation::ItemId).big_integer().not_null())
                    .col(ColumnDef::new(Annotation::Rating).integer().not_null())
                    .col(ColumnDef::new(Annotation::Starred).boolean().not_null())
                    .col(ColumnDef::new(Annotation::StarredAt).date_time().not_null())
                    .col(ColumnDef::new(Annotation::PlayedCount).integer().not_null())
                    .col(ColumnDef::new(Annotation::PlayedAt).date_time().not_null())
                    .col(ColumnDef::new(Annotation::Version).big_integer().not_null())
                    .col(ColumnDef::new(Annotation::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Annotation::UpdatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await?;

        // Create indexes for annotation table
        manager
            .create_index(
                Index::create()
                    .name("idx_annotation_user_id")
                    .table(Annotation::Table)
                    .col(Annotation::UserId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_annotation_item")
                    .table(Annotation::Table)
                    .col(Annotation::ItemKind)
                    .col(Annotation::ItemId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order
        manager
            .drop_table(Table::drop().table(Annotation::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PlaybackHistory::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum PlaybackHistory {
    Table,
    Id,
    UserId,
    AudioFileId,
    ScrobbledAt,
}

#[derive(DeriveIden)]
enum Annotation {
    Table,
    Id,
    UserId,
    ItemKind,
    ItemId,
    Rating,
    Starred,
    StarredAt,
    PlayedCount,
    PlayedAt,
    Version,
    CreatedAt,
    UpdatedAt,
}
