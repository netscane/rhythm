use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create genre table
        manager
            .create_table(
                Table::create()
                    .table(Genre::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Genre::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Genre::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(Genre::Version).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        // Create artist table
        manager
            .create_table(
                Table::create()
                    .table(Artist::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Artist::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Album::GenreId).big_integer().null())
                    .col(
                        ColumnDef::new(Album::GenreIds)
                            .array(ColumnType::BigInteger)
                            .null(),
                    )
                    .col(ColumnDef::new(Artist::Version).big_integer().not_null())
                    .col(ColumnDef::new(Artist::Name).string().not_null())
                    .col(ColumnDef::new(Artist::SortName).string().not_null())
                    .col(
                        ColumnDef::new(Artist::CreateTime)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Artist::UpdateTime)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create album table
        manager
            .create_table(
                Table::create()
                    .table(Album::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Album::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Album::Version).big_integer().not_null())
                    .col(ColumnDef::new(Album::Name).string().not_null())
                    .col(ColumnDef::new(Album::PathProtocol).string().not_null())
                    .col(ColumnDef::new(Album::PathPath).string().not_null())
                    .col(ColumnDef::new(Album::ArtistId).big_integer().not_null())
                    .col(ColumnDef::new(Album::GenreId).big_integer().null())
                    .col(
                        ColumnDef::new(Album::GenreIds)
                            .array(ColumnType::BigInteger)
                            .null(),
                    )
                    .col(ColumnDef::new(Album::MaxYear).integer().null())
                    .col(ColumnDef::new(Album::MinYear).integer().null())
                    .col(ColumnDef::new(Album::MaxOriginalYear).integer().null())
                    .col(ColumnDef::new(Album::MinOriginalYear).integer().null())
                    .col(ColumnDef::new(Album::Date).string().null())
                    .col(ColumnDef::new(Album::OriginalDate).string().null())
                    .col(ColumnDef::new(Album::ReleaseDate).string().null())
                    .col(ColumnDef::new(Album::Releases).integer().null())
                    .col(ColumnDef::new(Album::Compilation).boolean().not_null())
                    .col(ColumnDef::new(Album::SortName).string().not_null())
                    .col(ColumnDef::new(Album::OrderName).string().null())
                    .col(ColumnDef::new(Album::CatalogNum).string().null())
                    .col(ColumnDef::new(Album::Description).string().null())
                    .col(ColumnDef::new(Album::CreateTime).date_time().not_null())
                    .col(ColumnDef::new(Album::UpdateTime).date_time().not_null())
                    .to_owned(),
            )
            .await?;

        // Create artist_genre table
        manager
            .create_table(
                Table::create()
                    .table(ArtistGenre::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ArtistGenre::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ArtistGenre::ArtistId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArtistGenre::GenreId)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create album_genre table
        manager
            .create_table(
                Table::create()
                    .table(AlbumGenre::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AlbumGenre::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AlbumGenre::AlbumId).big_integer().not_null())
                    .col(ColumnDef::new(AlbumGenre::GenreId).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        // Create participant table
        manager
            .create_table(
                Table::create()
                    .table(Participant::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Participant::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(
                        ColumnDef::new(Participant::ArtistId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Participant::Role).string().not_null())
                    .col(ColumnDef::new(Participant::SubRole).string().null())
                    .col(ColumnDef::new(Participant::WorkId).big_integer().not_null())
                    .col(ColumnDef::new(Participant::WorkType).string().not_null())
                    .col(
                        ColumnDef::new(Participant::CreateTime)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Participant::UpdateTime)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique constraint for participant table
        manager
            .create_index(
                Index::create()
                    .name("participant_work_artist_role_unique")
                    .table(Participant::Table)
                    .col(Participant::WorkId)
                    .col(Participant::WorkType)
                    .col(Participant::ArtistId)
                    .col(Participant::Role)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create indexes for artist_genre table
        manager
            .create_index(
                Index::create()
                    .name("idx_artist_genre_artist_id")
                    .table(ArtistGenre::Table)
                    .col(ArtistGenre::ArtistId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_artist_genre_genre_id")
                    .table(ArtistGenre::Table)
                    .col(ArtistGenre::GenreId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create indexes for album_genre table
        manager
            .create_index(
                Index::create()
                    .name("idx_album_genre_album_id")
                    .table(AlbumGenre::Table)
                    .col(AlbumGenre::AlbumId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_album_genre_genre_id")
                    .table(AlbumGenre::Table)
                    .col(AlbumGenre::GenreId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create indexes for participant table
        manager
            .create_index(
                Index::create()
                    .name("idx_participant_artist_id_work_id_work_type")
                    .table(Participant::Table)
                    .col(Participant::ArtistId)
                    .col(Participant::WorkId)
                    .col(Participant::WorkType)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_participant_work_id_work_type")
                    .table(Participant::Table)
                    .col(Participant::WorkId)
                    .col(Participant::WorkType)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create audio_file table
        manager
            .create_table(
                Table::create()
                    .table(AudioFile::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AudioFile::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AudioFile::LibraryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AudioFile::GenreId).big_integer().null())
                    .col(
                        ColumnDef::new(AudioFile::GenreIds)
                            .array(ColumnType::BigInteger)
                            .null(),
                    )
                    .col(ColumnDef::new(AudioFile::PathProtocol).string().not_null())
                    .col(ColumnDef::new(AudioFile::PathPath).string().not_null())
                    .col(ColumnDef::new(AudioFile::Size).big_integer().not_null())
                    .col(ColumnDef::new(AudioFile::Suffix).string().not_null())
                    .col(ColumnDef::new(AudioFile::Hash).string().null())
                    .col(ColumnDef::new(AudioFile::Duration).big_integer().not_null())
                    .col(ColumnDef::new(AudioFile::BitRate).integer().not_null())
                    .col(ColumnDef::new(AudioFile::BitDepth).integer().not_null())
                    .col(ColumnDef::new(AudioFile::SampleRate).integer().not_null())
                    .col(ColumnDef::new(AudioFile::Channels).integer().not_null())
                    .col(ColumnDef::new(AudioFile::HasCoverArt).boolean().not_null())
                    .col(ColumnDef::new(AudioFile::Title).string().not_null())
                    .col(ColumnDef::new(AudioFile::AlbumId).big_integer().null())
                    .col(ColumnDef::new(AudioFile::ArtistId).big_integer().null())
                    .col(ColumnDef::new(AudioFile::TrackNumber).integer().null())
                    .col(ColumnDef::new(AudioFile::DiscNumber).integer().null())
                    .col(ColumnDef::new(AudioFile::Year).integer().null())
                    .col(ColumnDef::new(AudioFile::Date).integer().null())
                    .col(ColumnDef::new(AudioFile::OriginalYear).integer().null())
                    .col(ColumnDef::new(AudioFile::OriginalDate).integer().null())
                    .col(ColumnDef::new(AudioFile::ReleaseYear).integer().null())
                    .col(ColumnDef::new(AudioFile::ReleaseDate).integer().null())
                    .col(ColumnDef::new(AudioFile::Compilation).boolean().not_null())
                    .col(ColumnDef::new(AudioFile::Bpm).integer().null())
                    .col(ColumnDef::new(AudioFile::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(AudioFile::UpdatedAt).date_time().not_null())
                    .col(ColumnDef::new(AudioFile::Version).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        // Create index for audio_file.artist_id
        manager
            .create_index(
                Index::create()
                    .name("idx_audio_file_artist_id")
                    .table(AudioFile::Table)
                    .col(AudioFile::ArtistId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create trigger function for artist update_time
        let create_function_sql = r#"
            CREATE OR REPLACE FUNCTION update_artist_update_time_column()
            RETURNS TRIGGER AS $$
            BEGIN
                NEW.update_time = CURRENT_TIMESTAMP;
                RETURN NEW;
            END;
            $$ LANGUAGE plpgsql;
        "#;

        manager
            .get_connection()
            .execute_unprepared(create_function_sql)
            .await?;

        // Create trigger for artist table
        let create_trigger_sql = r#"
            CREATE TRIGGER update_artist_update_time
            BEFORE UPDATE ON artist
            FOR EACH ROW
            EXECUTE FUNCTION update_artist_update_time_column();
        "#;

        manager
            .get_connection()
            .execute_unprepared(create_trigger_sql)
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop trigger
        manager
            .get_connection()
            .execute_unprepared("DROP TRIGGER IF EXISTS update_artist_update_time ON artist;")
            .await?;

        // Drop trigger function
        manager
            .get_connection()
            .execute_unprepared("DROP FUNCTION IF EXISTS update_artist_update_time_column();")
            .await?;

        // Drop tables in reverse order
        manager
            .drop_table(Table::drop().table(AudioFile::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Participant::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(AlbumGenre::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ArtistGenre::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Album::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Artist::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Genre::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Genre {
    Table,
    Id,
    Name,
    Version,
}

#[derive(DeriveIden)]
enum ArtistGenre {
    Table,
    Id,
    ArtistId,
    GenreId,
}

#[derive(DeriveIden)]
enum AlbumGenre {
    Table,
    Id,
    AlbumId,
    GenreId,
}

#[derive(DeriveIden)]
enum Artist {
    Table,
    Id,
    GenreId,
    GenreIds,
    Version,
    Name,
    SortName,
    CreateTime,
    UpdateTime,
}

#[derive(DeriveIden)]
enum Album {
    Table,
    Id,
    Version,
    Name,
    PathProtocol,
    PathPath,
    ArtistId,
    GenreId,
    GenreIds,
    MaxYear,
    MinYear,
    Date,
    MaxOriginalYear,
    MinOriginalYear,
    OriginalDate,
    ReleaseDate,
    Releases,
    Compilation,
    SortName,
    OrderName,
    CatalogNum,
    Description,
    CreateTime,
    UpdateTime,
}

#[derive(DeriveIden)]
enum Participant {
    Table,
    Id,
    ArtistId,
    Role,
    SubRole,
    WorkId,
    WorkType,
    CreateTime,
    UpdateTime,
}

#[derive(DeriveIden)]
enum AudioFile {
    Table,
    Id,
    LibraryId,
    GenreId,
    GenreIds,
    PathProtocol,
    PathPath,
    Size,
    Suffix,
    Hash,
    Duration,
    BitRate,
    BitDepth,
    SampleRate,
    Channels,
    HasCoverArt,
    Title,
    AlbumId,
    ArtistId,
    TrackNumber,
    DiscNumber,
    Year,
    Date,
    OriginalYear,
    OriginalDate,
    ReleaseYear,
    ReleaseDate,
    Compilation,
    Bpm,
    CreatedAt,
    UpdatedAt,
    Version,
}
