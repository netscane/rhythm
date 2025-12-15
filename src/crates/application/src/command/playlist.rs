use std::sync::Arc;

use super::shared::IdGenerator;
use crate::error::AppError;
use domain::playlist::{Owner, Playlist, PlaylistError, PlaylistRepository};
use domain::value::{PlaylistId, UserId};

/// 创建或更新播放列表命令
#[derive(Debug)]
pub struct CreatePlaylistCmd {
    pub playlist_id: Option<i64>,
    pub name: Option<String>,
    pub owner_id: i64,
    pub owner_name: String,
    pub song_ids: Vec<i64>,
}

/// 更新播放列表命令
#[derive(Debug)]
pub struct UpdatePlaylistCmd {
    pub playlist_id: i64,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub public: Option<bool>,
    pub song_ids_to_add: Vec<i64>,
    pub song_indexes_to_remove: Vec<usize>,
}

/// 播放列表应用服务
pub struct PlaylistAppService {
    playlist_repository: Arc<dyn PlaylistRepository>,
    id_generator: Arc<dyn IdGenerator>,
}

impl PlaylistAppService {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            playlist_repository,
            id_generator,
        }
    }

    /// 创建或更新播放列表
    pub async fn create_playlist(&self, cmd: CreatePlaylistCmd) -> Result<Playlist, AppError> {
        let mut playlist = if let Some(playlist_id) = cmd.playlist_id {
            // 更新现有播放列表
            let mut playlist = self
                .playlist_repository
                .find_by_id(PlaylistId::from(playlist_id))
                .await
                .map_err(|e| AppError::UnknownError(e.to_string()))?
                .ok_or_else(|| {
                    AppError::AggregateNotFound(
                        "Playlist".to_string(),
                        format!("id {} not found", playlist_id),
                    )
                })?;

            if let Some(name) = cmd.name {
                playlist.update_name(&name);
            }

            playlist
        } else {
            // 创建新播放列表
            let name = cmd.name.ok_or_else(|| {
                AppError::InvalidInput("name is required when creating a playlist".to_string())
            })?;

            let playlist_id = PlaylistId::from(self.id_generator.next_id().await?);
            let owner = Owner {
                id: UserId::from(cmd.owner_id),
                name: cmd.owner_name,
            };

            Playlist::new(playlist_id, &name, owner, None, false)
        };

        // 添加歌曲
        for song_id in cmd.song_ids {
            let entry_id = self.id_generator.next_id().await?;
            playlist.add_entry(entry_id, song_id);
        }

        // 保存
        self.playlist_repository
            .save(&mut playlist)
            .await
            .map_err(|e| AppError::UnknownError(e.to_string()))?;

        Ok(playlist)
    }

    /// 删除播放列表
    pub async fn delete_playlist(&self, playlist_id: i64) -> Result<(), AppError> {
        self.playlist_repository
            .delete(PlaylistId::from(playlist_id))
            .await
            .map_err(|e| AppError::UnknownError(e.to_string()))?;

        Ok(())
    }

    /// 更新播放列表
    pub async fn update_playlist(&self, cmd: UpdatePlaylistCmd) -> Result<(), AppError> {
        let mut playlist = self
            .playlist_repository
            .find_by_id(PlaylistId::from(cmd.playlist_id))
            .await
            .map_err(|e| AppError::UnknownError(e.to_string()))?
            .ok_or_else(|| {
                AppError::AggregateNotFound(
                    "Playlist".to_string(),
                    format!("id {} not found", cmd.playlist_id),
                )
            })?;

        // 更新名称
        if let Some(name) = cmd.name {
            playlist.update_name(&name);
        }

        // 更新备注
        if let Some(comment) = cmd.comment {
            playlist.update_comment(Some(&comment));
        }

        // 更新公开状态
        if let Some(public) = cmd.public {
            playlist.set_public(public);
        }

        // 按索引删除歌曲（从大到小排序，避免索引偏移问题）
        let mut indexes_to_remove = cmd.song_indexes_to_remove;
        indexes_to_remove.sort_by(|a, b| b.cmp(a));
        for index in indexes_to_remove {
            if index < playlist.entries.len() {
                playlist.entries.remove(index);
            }
        }

        // 添加歌曲
        for song_id in cmd.song_ids_to_add {
            let entry_id = self.id_generator.next_id().await?;
            playlist.add_entry(entry_id, song_id);
        }

        // 保存
        self.playlist_repository
            .save(&mut playlist)
            .await
            .map_err(|e| AppError::UnknownError(e.to_string()))?;

        Ok(())
    }
}

impl From<PlaylistError> for AppError {
    fn from(e: PlaylistError) -> Self {
        match e {
            PlaylistError::DbErr(msg) => AppError::RepositoryError("Playlist".to_string(), msg),
            PlaylistError::RepositoryErr(msg) => {
                AppError::RepositoryError("Playlist".to_string(), msg)
            }
            PlaylistError::ValidationErr(msg) => AppError::InvalidInput(msg),
            PlaylistError::EntryNotFound(id) => {
                AppError::AggregateNotFound("PlaylistEntry".to_string(), id.to_string())
            }
            PlaylistError::OtherErr(msg) => AppError::UnknownError(msg),
        }
    }
}
