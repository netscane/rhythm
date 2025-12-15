use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create cover_art table
        manager
            .create_table(
                Table::create()
                    .table(CoverArt::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CoverArt::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CoverArt::Version).big_integer().not_null())
                    .col(ColumnDef::new(CoverArt::AudioFileId).big_integer().null())
                    .col(ColumnDef::new(CoverArt::SongId).big_integer().null())
                    .col(ColumnDef::new(CoverArt::AlbumId).big_integer().null())
                    .col(ColumnDef::new(CoverArt::PathProtocol).string().not_null())
                    .col(ColumnDef::new(CoverArt::PathPath).string().not_null())
                    .col(ColumnDef::new(CoverArt::Width).integer().null())
                    .col(ColumnDef::new(CoverArt::Height).integer().null())
                    .col(ColumnDef::new(CoverArt::Format).string().null())
                    .col(ColumnDef::new(CoverArt::FileSize).big_integer().not_null())
                    .col(ColumnDef::new(CoverArt::Source).string().not_null())
                    .col(ColumnDef::new(CoverArt::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(CoverArt::UpdatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await?;

        // NOTE: album_cover_art and artist_cover_art tables removed
        // Cover art is now determined dynamically at query time instead of pre-computed

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop cover_art table
        manager
            .drop_table(Table::drop().table(CoverArt::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum CoverArt {
    Table,
    Id,
    Version,
    AudioFileId,
    SongId,
    AlbumId,
    PathProtocol,
    PathPath,
    Width,
    Height,
    Format,
    FileSize,
    Source,
    CreatedAt,
    UpdatedAt,
}
