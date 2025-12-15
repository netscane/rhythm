use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create user table
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(User::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(User::Username)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(User::Name).string().not_null())
                    .col(ColumnDef::new(User::Email).string().not_null())
                    .col(ColumnDef::new(User::Password).string().not_null())
                    .col(ColumnDef::new(User::EncryptedPassword).string().not_null().default(""))
                    .col(ColumnDef::new(User::IsAdmin).boolean().not_null())
                    .col(ColumnDef::new(User::Status).integer().not_null())
                    .col(ColumnDef::new(User::LastLoginAt).date_time().not_null())
                    .col(ColumnDef::new(User::LastAccessAt).date_time().not_null())
                    .col(ColumnDef::new(User::LastOpTime).date_time().not_null())
                    .col(ColumnDef::new(User::Version).big_integer().not_null())
                    .col(ColumnDef::new(User::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(User::UpdatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await?;

        // Create library table
        manager
            .create_table(
                Table::create()
                    .table(Library::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Library::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Library::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Library::PathProtocol).string().not_null())
                    .col(ColumnDef::new(Library::PathPath).string().not_null())
                    .col(ColumnDef::new(Library::ScanStatus).integer().not_null())
                    .col(ColumnDef::new(Library::LastScanAt).date_time().not_null())
                    .col(ColumnDef::new(Library::Version).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        // Create library_item table
        manager
            .create_table(
                Table::create()
                    .table(LibraryItem::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LibraryItem::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LibraryItem::LibraryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LibraryItem::PathProtocol)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(LibraryItem::PathPath).string().not_null())
                    .col(ColumnDef::new(LibraryItem::Size).big_integer().not_null())
                    .col(ColumnDef::new(LibraryItem::Suffix).string().not_null())
                    .col(ColumnDef::new(LibraryItem::Mtime).date_time().not_null())
                    .col(ColumnDef::new(LibraryItem::Atime).date_time().not_null())
                    .col(ColumnDef::new(LibraryItem::State).integer().not_null())
                    .col(ColumnDef::new(LibraryItem::FileType).string().not_null())
                    .to_owned(),
            )
            .await?;

        // Create system_config table
        manager
            .create_table(
                Table::create()
                    .table(SystemConfig::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SystemConfig::Key)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SystemConfig::Value).text().not_null())
                    .col(
                        ColumnDef::new(SystemConfig::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SystemConfig::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order
        manager
            .drop_table(Table::drop().table(SystemConfig::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(LibraryItem::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Library::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Username,
    Name,
    Email,
    Password,
    EncryptedPassword,
    IsAdmin,
    Status,
    LastLoginAt,
    LastAccessAt,
    LastOpTime,
    Version,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Library {
    Table,
    Id,
    Name,
    PathProtocol,
    PathPath,
    ScanStatus,
    LastScanAt,
    Version,
}

#[derive(DeriveIden)]
enum LibraryItem {
    Table,
    Id,
    LibraryId,
    PathProtocol,
    PathPath,
    Size,
    Suffix,
    Mtime,
    Atime,
    State,
    FileType,
}

#[derive(DeriveIden)]
enum SystemConfig {
    Table,
    Key,
    Value,
    CreatedAt,
    UpdatedAt,
}
