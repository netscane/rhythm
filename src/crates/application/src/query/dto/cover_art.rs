use super::artwork_id::{ArtworkId, ArtworkKind};

/// Cover Art ID 生成辅助函数
/// 统一管理 cover_art_id 的生成规则
///
/// **注意**: 推荐使用 `ArtworkId` 结构体以获得更好的类型安全和功能
/// 这些简化函数仅用于快速迁移，最终应该迁移到 `ArtworkId`

/// 为艺术家生成 cover_art_id (字符串格式)
/// 格式：ar-{artist_id}
///
/// 推荐使用: `ArtworkId::for_artist(artist_id).to_string()`
pub fn artist_cover_art_id(artist_id: i64) -> String {
    ArtworkId::for_artist(artist_id).to_string()
}

/// 为专辑生成 cover_art_id (字符串格式)
/// 格式：al-{album_id}
///
/// 推荐使用: `ArtworkId::for_album(album_id).to_string()`
pub fn album_cover_art_id(album_id: i64) -> String {
    ArtworkId::for_album(album_id).to_string()
}

/// 为音频文件生成 cover_art_id
pub fn audio_file_cover_art_id(file_id: i64) -> String {
    ArtworkId::for_audio_file(file_id).to_string()
}

/// 为播放列表生成 cover_art_id
pub fn playlist_cover_art_id(playlist_id: i64) -> String {
    ArtworkId::for_playlist(playlist_id).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artist_cover_art_id() {
        assert_eq!(artist_cover_art_id(123), "ar-123");
    }

    #[test]
    fn test_album_cover_art_id() {
        assert_eq!(album_cover_art_id(456), "al-456");
    }
}
