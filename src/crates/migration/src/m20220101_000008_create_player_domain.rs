use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create player table
        manager
            .create_table(
                Table::create()
                    .table(Player::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Player::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Player::Name).string().not_null())
                    .col(ColumnDef::new(Player::UserAgent).string().not_null())
                    .col(ColumnDef::new(Player::UserId).big_integer().not_null())
                    .col(ColumnDef::new(Player::Client).string().not_null())
                    .col(ColumnDef::new(Player::Ip).string().not_null())
                    .col(ColumnDef::new(Player::LastSeen).date_time().not_null())
                    .col(ColumnDef::new(Player::MaxBitRate).integer().not_null())
                    .col(ColumnDef::new(Player::TranscodingId).string().not_null())
                    .col(
                        ColumnDef::new(Player::ReportRealPath)
                            .tiny_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Player::ScrobbleEnabled)
                            .tiny_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Player::Version).integer().not_null())
                    .col(ColumnDef::new(Player::LastOpTime).date_time().not_null())
                    .col(ColumnDef::new(Player::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Player::UpdatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_player_user_id")
                            .from(Player::Table, Player::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Player::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Player {
    Table,
    Id,
    Name,
    UserAgent,
    UserId,
    Client,
    Ip,
    LastSeen,
    MaxBitRate,
    TranscodingId,
    ReportRealPath,
    ScrobbleEnabled,
    Version,
    LastOpTime,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}
