use std::fmt;

/// Artwork 类型（参考 Navidrome 设计）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtworkKind {
    /// 音频文件封面
    AudioFile,
    /// 艺术家封面
    Artist,
    /// 专辑封面
    Album,
    /// 播放列表封面
    Playlist,
}

impl ArtworkKind {
    /// 获取前缀
    pub fn prefix(&self) -> &'static str {
        match self {
            ArtworkKind::AudioFile => "af",
            ArtworkKind::Artist => "ar",
            ArtworkKind::Album => "al",
            ArtworkKind::Playlist => "pl",
        }
    }

    /// 从前缀解析
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix {
            "af" => Some(ArtworkKind::AudioFile),
            "ar" => Some(ArtworkKind::Artist),
            "al" => Some(ArtworkKind::Album),
            "pl" => Some(ArtworkKind::Playlist),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ArtworkKind::AudioFile => "audio_file",
            ArtworkKind::Artist => "artist",
            ArtworkKind::Album => "album",
            ArtworkKind::Playlist => "playlist",
        }
    }
}

impl fmt::Display for ArtworkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Artwork ID 结构（参考 Navidrome 设计）
/// 格式: {kind_prefix}-{id}
/// 例如: ar-123 或 al-456
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtworkId {
    pub kind: ArtworkKind,
    pub id: i64,
}

impl ArtworkId {
    /// 创建新的 ArtworkId
    pub fn new(kind: ArtworkKind, id: i64) -> Self {
        Self { kind, id }
    }

    /// 为艺术家创建 ArtworkId
    pub fn for_artist(artist_id: i64) -> Self {
        Self::new(ArtworkKind::Artist, artist_id)
    }

    /// 为专辑创建 ArtworkId
    pub fn for_album(album_id: i64) -> Self {
        Self::new(ArtworkKind::Album, album_id)
    }

    /// 为音频文件创建 ArtworkId
    pub fn for_audio_file(file_id: i64) -> Self {
        Self::new(ArtworkKind::AudioFile, file_id)
    }

    /// 为播放列表创建 ArtworkId
    pub fn for_playlist(playlist_id: i64) -> Self {
        Self::new(ArtworkKind::Playlist, playlist_id)
    }

    /// 转换为字符串格式
    /// 格式: {prefix}-{id}
    pub fn to_string(&self) -> String {
        format!("{}-{}", self.kind.prefix(), self.id)
    }

    /// 从字符串解析 ArtworkId
    /// 格式: {prefix}-{id}
    pub fn parse(s: &str) -> Result<Self, ArtworkIdParseError> {
        // 分割前缀和 ID
        let parts: Vec<&str> = s.splitn(2, '-').collect();
        if parts.len() != 2 {
            return Err(ArtworkIdParseError::InvalidFormat);
        }

        let kind = ArtworkKind::from_prefix(parts[0]).ok_or(ArtworkIdParseError::InvalidKind)?;

        let id = parts[1]
            .parse::<i64>()
            .map_err(|_| ArtworkIdParseError::InvalidId)?;

        Ok(Self { kind, id })
    }
}

impl fmt::Display for ArtworkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtworkIdParseError {
    InvalidFormat,
    InvalidKind,
    InvalidId,
}

impl fmt::Display for ArtworkIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtworkIdParseError::InvalidFormat => write!(f, "Invalid artwork ID format"),
            ArtworkIdParseError::InvalidKind => write!(f, "Invalid artwork kind"),
            ArtworkIdParseError::InvalidId => write!(f, "Invalid ID"),
        }
    }
}

impl std::error::Error for ArtworkIdParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artwork_kind_prefix() {
        assert_eq!(ArtworkKind::Artist.prefix(), "ar");
        assert_eq!(ArtworkKind::Album.prefix(), "al");
        assert_eq!(ArtworkKind::AudioFile.prefix(), "af");
        assert_eq!(ArtworkKind::Playlist.prefix(), "pl");
    }

    #[test]
    fn test_artwork_kind_from_prefix() {
        assert_eq!(ArtworkKind::from_prefix("ar"), Some(ArtworkKind::Artist));
        assert_eq!(ArtworkKind::from_prefix("al"), Some(ArtworkKind::Album));
        assert_eq!(ArtworkKind::from_prefix("xx"), None);
    }

    #[test]
    fn test_artwork_id_to_string() {
        let id = ArtworkId::for_artist(123);
        assert_eq!(id.to_string(), "ar-123");
    }

    #[test]
    fn test_artwork_id_to_string_album() {
        let id = ArtworkId::for_album(456);
        assert_eq!(id.to_string(), "al-456");
    }

    #[test]
    fn test_artwork_id_parse() {
        let result = ArtworkId::parse("ar-123").unwrap();
        assert_eq!(result.kind, ArtworkKind::Artist);
        assert_eq!(result.id, 123);
    }

    #[test]
    fn test_artwork_id_parse_album() {
        let result = ArtworkId::parse("al-456").unwrap();
        assert_eq!(result.kind, ArtworkKind::Album);
        assert_eq!(result.id, 456);
    }

    #[test]
    fn test_artwork_id_parse_invalid_format() {
        assert!(matches!(
            ArtworkId::parse("invalid"),
            Err(ArtworkIdParseError::InvalidFormat)
        ));
    }

    #[test]
    fn test_artwork_id_parse_invalid_kind() {
        assert!(matches!(
            ArtworkId::parse("xx-123"),
            Err(ArtworkIdParseError::InvalidKind)
        ));
    }

    #[test]
    fn test_artwork_id_roundtrip() {
        let original = ArtworkId::for_artist(789);
        let serialized = original.to_string();
        let parsed = ArtworkId::parse(&serialized).unwrap();
        assert_eq!(original, parsed);
    }
}
