use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create play_queue table
        // Each user has at most one play queue (1:1 relationship)
        manager
            .create_table(
                Table::create()
                    .table(PlayQueue::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayQueue::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PlayQueue::UserId).big_integer().not_null())
                    .col(ColumnDef::new(PlayQueue::CurrentId).big_integer().null())
                    .col(
                        ColumnDef::new(PlayQueue::Position)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(PlayQueue::ChangedBy)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(PlayQueue::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(PlayQueue::UpdatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_play_queue_user_id")
                            .from(PlayQueue::Table, PlayQueue::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index on user_id (each user has at most one play queue)
        manager
            .create_index(
                Index::create()
                    .name("idx_play_queue_user_id_unique")
                    .table(PlayQueue::Table)
                    .col(PlayQueue::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create play_queue_item table
        manager
            .create_table(
                Table::create()
                    .table(PlayQueueItem::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayQueueItem::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PlayQueueItem::PlayQueueId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayQueueItem::AudioFileId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayQueueItem::Position)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(PlayQueueItem::CreatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_play_queue_item_play_queue_id")
                            .from(PlayQueueItem::Table, PlayQueueItem::PlayQueueId)
                            .to(PlayQueue::Table, PlayQueue::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_play_queue_item_audio_file_id")
                            .from(PlayQueueItem::Table, PlayQueueItem::AudioFileId)
                            .to(AudioFile::Table, AudioFile::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on play_queue_item.play_queue_id
        manager
            .create_index(
                Index::create()
                    .name("idx_play_queue_item_play_queue_id")
                    .table(PlayQueueItem::Table)
                    .col(PlayQueueItem::PlayQueueId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlayQueueItem::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PlayQueue::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum PlayQueue {
    Table,
    Id,
    UserId,
    CurrentId,
    Position,
    ChangedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum PlayQueueItem {
    Table,
    Id,
    PlayQueueId,
    AudioFileId,
    Position,
    CreatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum AudioFile {
    Table,
    Id,
}
