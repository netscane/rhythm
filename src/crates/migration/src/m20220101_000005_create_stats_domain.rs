use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create album_stats table
        manager
            .create_table(
                Table::create()
                    .table(AlbumStats::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AlbumStats::AlbumId)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AlbumStats::Duration)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AlbumStats::Size).big_integer().not_null())
                    .col(ColumnDef::new(AlbumStats::SongCount).integer().not_null())
                    .col(ColumnDef::new(AlbumStats::Year).integer().null())
                    .col(
                        ColumnDef::new(AlbumStats::DiskNumbers)
                            .array(ColumnType::Integer)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index for album_stat table
        manager
            .create_index(
                Index::create()
                    .name("idx_album_stats_album_id")
                    .table(AlbumStats::Table)
                    .col(AlbumStats::AlbumId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create participant_stats table
        manager
            .create_table(
                Table::create()
                    .table(ParticipantStats::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ParticipantStats::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ParticipantStats::ArtistId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ParticipantStats::Role).string().not_null())
                    .col(
                        ColumnDef::new(ParticipantStats::Duration)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ParticipantStats::Size)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ParticipantStats::SongCount)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ParticipantStats::AlbumCount)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for participant_stats table
        // Unique constraint on (artist_id, role) combination
        manager
            .create_index(
                Index::create()
                    .name("idx_participant_stats_participant_id")
                    .table(ParticipantStats::Table)
                    .col(ParticipantStats::ArtistId)
                    .col(ParticipantStats::Role)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Non-unique index on artist_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_participant_stats_artist_id")
                    .table(ParticipantStats::Table)
                    .col(ParticipantStats::ArtistId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create genre_stats table
        manager
            .create_table(
                Table::create()
                    .table(GenreStats::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GenreStats::GenreId)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GenreStats::SongCount).integer().not_null())
                    .col(ColumnDef::new(GenreStats::AlbumCount).integer().not_null())
                    .to_owned(),
            )
            .await?;

        // Create index for genre_stats table
        manager
            .create_index(
                Index::create()
                    .name("idx_genre_stats_genre_id")
                    .table(GenreStats::Table)
                    .col(GenreStats::GenreId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order
        manager
            .drop_table(Table::drop().table(GenreStats::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ParticipantStats::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(AlbumStats::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum AlbumStats {
    Table,
    AlbumId,
    Duration,
    Size,
    SongCount,
    Year,
    DiskNumbers,
}

#[derive(DeriveIden)]
enum GenreStats {
    Table,
    GenreId,
    SongCount,
    AlbumCount,
}

#[derive(DeriveIden)]
enum ParticipantStats {
    Table,
    Id,
    ArtistId,
    Role,
    Duration,
    Size,
    SongCount,
    AlbumCount,
}
