use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create playlist table
        manager
            .create_table(
                Table::create()
                    .table(Playlist::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Playlist::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Playlist::Name).string().not_null())
                    .col(ColumnDef::new(Playlist::Comment).string().null())
                    .col(ColumnDef::new(Playlist::OwnerId).big_integer().not_null())
                    .col(ColumnDef::new(Playlist::OwnerName).string().not_null())
                    .col(ColumnDef::new(Playlist::Public).boolean().not_null().default(false))
                    .col(ColumnDef::new(Playlist::Version).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Playlist::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Playlist::UpdatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_playlist_owner_id")
                            .from(Playlist::Table, Playlist::OwnerId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create playlist_entry table
        manager
            .create_table(
                Table::create()
                    .table(PlaylistEntry::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlaylistEntry::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PlaylistEntry::PlaylistId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlaylistEntry::AudioFileId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlaylistEntry::Position)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(PlaylistEntry::AddedAt).date_time().not_null())
                    .col(ColumnDef::new(PlaylistEntry::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(PlaylistEntry::UpdatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_playlist_entry_playlist_id")
                            .from(PlaylistEntry::Table, PlaylistEntry::PlaylistId)
                            .to(Playlist::Table, Playlist::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_playlist_entry_audio_file_id")
                            .from(PlaylistEntry::Table, PlaylistEntry::AudioFileId)
                            .to(AudioFile::Table, AudioFile::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on playlist_entry.playlist_id
        manager
            .create_index(
                Index::create()
                    .name("idx_playlist_entry_playlist_id")
                    .table(PlaylistEntry::Table)
                    .col(PlaylistEntry::PlaylistId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlaylistEntry::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Playlist::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Playlist {
    Table,
    Id,
    Name,
    Comment,
    OwnerId,
    OwnerName,
    Public,
    Version,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum PlaylistEntry {
    Table,
    Id,
    PlaylistId,
    AudioFileId,
    Position,
    AddedAt,
    CreatedAt,
    UpdatedAt,
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
