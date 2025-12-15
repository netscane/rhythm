use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create artist_location table
        manager
            .create_table(
                Table::create()
                    .table(ArtistLocation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ArtistLocation::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ArtistLocation::ArtistId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArtistLocation::LocationProtocol)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArtistLocation::LocationPath)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArtistLocation::Total)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArtistLocation::UpdateTime)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for artist_location table
        manager
            .create_index(
                Index::create()
                    .name("idx_artist_location_artist_id")
                    .table(ArtistLocation::Table)
                    .col(ArtistLocation::ArtistId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_artist_location_protocol_path")
                    .table(ArtistLocation::Table)
                    .col(ArtistLocation::LocationProtocol)
                    .col(ArtistLocation::LocationPath)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_artist_location_unique")
                    .table(ArtistLocation::Table)
                    .col(ArtistLocation::ArtistId)
                    .col(ArtistLocation::LocationProtocol)
                    .col(ArtistLocation::LocationPath)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create album_location table
        manager
            .create_table(
                Table::create()
                    .table(AlbumLocation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AlbumLocation::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AlbumLocation::AlbumId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AlbumLocation::LocationProtocol)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AlbumLocation::LocationPath)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AlbumLocation::Total)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AlbumLocation::UpdateTime)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for album_location table
        manager
            .create_index(
                Index::create()
                    .name("idx_album_location_album_id")
                    .table(AlbumLocation::Table)
                    .col(AlbumLocation::AlbumId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_album_location_protocol_path")
                    .table(AlbumLocation::Table)
                    .col(AlbumLocation::LocationProtocol)
                    .col(AlbumLocation::LocationPath)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_album_location_unique")
                    .table(AlbumLocation::Table)
                    .col(AlbumLocation::AlbumId)
                    .col(AlbumLocation::LocationProtocol)
                    .col(AlbumLocation::LocationPath)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create trigger function for update_time column
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE OR REPLACE FUNCTION update_update_time_column()
                RETURNS TRIGGER AS $$
                BEGIN
                    NEW.update_time = CURRENT_TIMESTAMP;
                    RETURN NEW;
                END;
                $$ language 'plpgsql';
                "#,
            )
            .await?;

        // Create triggers
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TRIGGER update_artist_location_update_time
                BEFORE UPDATE ON artist_location
                FOR EACH ROW
                EXECUTE FUNCTION update_update_time_column();
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TRIGGER update_album_location_update_time
                BEFORE UPDATE ON album_location
                FOR EACH ROW
                EXECUTE FUNCTION update_update_time_column();
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop triggers
        manager
            .get_connection()
            .execute_unprepared("DROP TRIGGER IF EXISTS update_album_location_update_time ON album_location;")
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP TRIGGER IF EXISTS update_artist_location_update_time ON artist_location;")
            .await?;

        // Drop tables
        manager
            .drop_table(Table::drop().table(AlbumLocation::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ArtistLocation::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ArtistLocation {
    Table,
    Id,
    ArtistId,
    LocationProtocol,
    LocationPath,
    Total,
    UpdateTime,
}

#[derive(DeriveIden)]
enum AlbumLocation {
    Table,
    Id,
    AlbumId,
    LocationProtocol,
    LocationPath,
    Total,
    UpdateTime,
}
