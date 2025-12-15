use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Transcoding::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Transcoding::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Transcoding::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Transcoding::TargetFormat)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Transcoding::Command).string().not_null())
                    .col(
                        ColumnDef::new(Transcoding::DefaultBitRate)
                            .integer()
                            .not_null()
                            .default(128),
                    )
                    .col(
                        ColumnDef::new(Transcoding::Parameters)
                            .string()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(Transcoding::Version)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_transcoding_target_format")
                    .table(Transcoding::Table)
                    .col(Transcoding::TargetFormat)
                    .to_owned(),
            )
            .await?;

        // 插入种子数据
        self.seed_transcoding_data(manager).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Transcoding::Table).to_owned())
            .await
    }
}

impl Migration {
    async fn seed_transcoding_data(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // MP3 转码配置
        db.execute_unprepared(
            r#"INSERT INTO transcoding (id, name, target_format, command, default_bit_rate, parameters, version)
               VALUES (1, 'MP3 Audio', 'mp3', 'ffmpeg -i %s -map 0:a:0 -b:a %bk -v 0 -f mp3 -', 192, '{}', 1)
               ON CONFLICT (id) DO NOTHING"#,
        )
        .await?;

        // AAC 转码配置
        db.execute_unprepared(
            r#"INSERT INTO transcoding (id, name, target_format, command, default_bit_rate, parameters, version)
               VALUES (2, 'AAC Audio', 'aac', 'ffmpeg -i %s -map 0:a:0 -b:a %bk -v 0 -c:a aac -f adts -', 192, '{}', 1)
               ON CONFLICT (id) DO NOTHING"#,
        )
        .await?;

        // Opus 转码配置
        db.execute_unprepared(
            r#"INSERT INTO transcoding (id, name, target_format, command, default_bit_rate, parameters, version)
               VALUES (3, 'Opus Audio', 'opus', 'ffmpeg -i %s -map 0:a:0 -b:a %bk -v 0 -c:a libopus -f opus -', 128, '{}', 1)
               ON CONFLICT (id) DO NOTHING"#,
        )
        .await?;

        // OGG Vorbis 转码配置
        db.execute_unprepared(
            r#"INSERT INTO transcoding (id, name, target_format, command, default_bit_rate, parameters, version)
               VALUES (4, 'OGG Vorbis', 'ogg', 'ffmpeg -i %s -map 0:a:0 -b:a %bk -v 0 -c:a libvorbis -f ogg -', 160, '{}', 1)
               ON CONFLICT (id) DO NOTHING"#,
        )
        .await?;

        // FLAC 转码配置（无损）
        db.execute_unprepared(
            r#"INSERT INTO transcoding (id, name, target_format, command, default_bit_rate, parameters, version)
               VALUES (5, 'FLAC Audio', 'flac', 'ffmpeg -i %s -map 0:a:0 -v 0 -c:a flac -f flac -', 0, '{}', 1)
               ON CONFLICT (id) DO NOTHING"#,
        )
        .await?;

        // WAV 转码配置（无损）
        db.execute_unprepared(
            r#"INSERT INTO transcoding (id, name, target_format, command, default_bit_rate, parameters, version)
               VALUES (6, 'WAV Audio', 'wav', 'ffmpeg -i %s -map 0:a:0 -v 0 -c:a pcm_s16le -f wav -', 0, '{}', 1)
               ON CONFLICT (id) DO NOTHING"#,
        )
        .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Transcoding {
    Table,
    Id,
    Name,
    TargetFormat,
    Command,
    DefaultBitRate,
    Parameters,
    Version,
}
