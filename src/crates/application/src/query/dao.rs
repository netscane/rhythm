use crate::query::QueryError;
use async_trait::async_trait;
use model::album::{Album, AlbumInfo};
use model::artist::{Artist, ArtistInfo};
use model::audio_file::AudioFile;
use model::genre::Genre;
use model::music_folder::MusicFolder;
use model::play_queue::PlayQueue;
use model::playlist::{Playlist, PlaylistSummary};

#[async_trait]
pub trait MusicFolderDao {
    async fn get_by_id(&self, id: i64) -> Result<Option<MusicFolder>, QueryError>;
    async fn get_all(&self) -> Result<Vec<MusicFolder>, QueryError>;
}

#[async_trait]
pub trait ArtistDao {
    async fn get_by_id(&self, id: i64) -> Result<Option<Artist>, QueryError>;
    async fn get_all(&self) -> Result<Vec<Artist>, QueryError>;
    async fn get_artist_info(&self, artist_id: i64) -> Result<Option<ArtistInfo>, QueryError>;
    /// 根据 sort_name 查询艺术家（优先匹配 sort_name，如果没有则匹配 name）
    async fn get_by_sort_name(&self, artist_name: &str) -> Result<Option<Artist>, QueryError>;
    /// 获取随机艺术家 ID（排除指定的 artist_id，如果为 None 则不排除）
    async fn get_random_artist_id(
        &self,
        exclude_artist_id: Option<i64>,
    ) -> Result<Option<i64>, QueryError>;
    /// 查询已收藏的艺术家列表（按用户 ID 过滤）
    async fn get_by_starred(&self, user_id: i64) -> Result<Vec<Artist>, QueryError>;
    /// 搜索艺术家（支持分页）
    async fn search(
        &self,
        query: &str,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<Artist>, QueryError>;
    /// 获取播放次数最多的艺术家
    async fn get_most_played(&self, limit: i32) -> Result<Vec<Artist>, QueryError>;
    /// 获取最近播放的艺术家
    async fn get_recently_played(&self, limit: i32) -> Result<Vec<Artist>, QueryError>;
}

#[async_trait]
pub trait GenreDao {
    async fn get_all(&self) -> Result<Vec<Genre>, QueryError>;
}

#[async_trait]
pub trait AlbumDao {
    async fn get_by_id(&self, id: i64) -> Result<Option<Album>, QueryError>;
    async fn get_by_artist_id(&self, artist_id: i64) -> Result<Vec<Album>, QueryError>;
    async fn get_all(&self) -> Result<Vec<Album>, QueryError>;
    async fn get_album_info(&self, album_id: i64) -> Result<Option<AlbumInfo>, QueryError>;
    /// 查询最新专辑列表（按创建时间降序）
    async fn get_by_newest(&self, offset: i32, limit: i32)
        -> Result<(Vec<Album>, i64), QueryError>;
    /// 查询最近播放的专辑列表（按播放时间降序）
    async fn get_by_recent(&self, offset: i32, limit: i32)
        -> Result<(Vec<Album>, i64), QueryError>;
    /// 查询随机专辑列表
    async fn get_by_random(&self, offset: i32, limit: i32)
        -> Result<(Vec<Album>, i64), QueryError>;
    /// 查询专辑列表（按名称字母顺序）
    async fn get_by_name(&self, offset: i32, limit: i32) -> Result<(Vec<Album>, i64), QueryError>;
    /// 查询专辑列表（按艺术家字母顺序）
    async fn get_by_artist(&self, offset: i32, limit: i32)
        -> Result<(Vec<Album>, i64), QueryError>;
    /// 查询最常播放的专辑列表（按播放次数降序）
    async fn get_by_frequent(
        &self,
        offset: i32,
        limit: i32,
    ) -> Result<(Vec<Album>, i64), QueryError>;
    /// 查询已收藏的专辑列表（支持分页，按用户 ID 过滤）
    async fn get_by_starred(
        &self,
        user_id: i64,
        offset: i32,
        limit: i32,
    ) -> Result<(Vec<Album>, i64), QueryError>;
    /// 查询所有已收藏的专辑列表（无分页，按用户 ID 过滤）
    async fn get_starred(&self, user_id: i64) -> Result<Vec<Album>, QueryError>;
    /// 查询高评分专辑列表（按评分降序）
    async fn get_by_rating(&self, offset: i32, limit: i32)
        -> Result<(Vec<Album>, i64), QueryError>;
    /// 根据流派查询专辑列表
    async fn get_by_genre(
        &self,
        genre: &str,
        offset: i32,
        limit: i32,
    ) -> Result<(Vec<Album>, i64), QueryError>;
    /// 根据年份范围查询专辑列表
    async fn get_by_year(
        &self,
        from_year: i32,
        to_year: i32,
        offset: i32,
        limit: i32,
    ) -> Result<(Vec<Album>, i64), QueryError>;
    /// 搜索专辑（支持分页）
    async fn search(
        &self,
        query: &str,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<Album>, QueryError>;
}

#[async_trait]
pub trait AudioFileDao {
    async fn get_by_id(&self, id: i64) -> Result<Option<AudioFile>, QueryError>;
    async fn get_by_album_id(&self, album_id: i64) -> Result<Vec<AudioFile>, QueryError>;
    async fn get_by_artist_id(&self, artist_id: i64) -> Result<Vec<AudioFile>, QueryError>;
    async fn get_all(&self) -> Result<Vec<AudioFile>, QueryError>;
    /// 根据艺术家 ID 查询 top songs（按播放次数排序）
    async fn get_top_songs_by_artist_id(
        &self,
        artist_id: i64,
        limit: i32,
    ) -> Result<Vec<AudioFile>, QueryError>;
    /// 查询随机歌曲（可选流派和年份范围过滤）
    async fn get_random_songs(
        &self,
        genre: Option<&str>,
        from_year: Option<i32>,
        to_year: Option<i32>,
        limit: i32,
    ) -> Result<Vec<AudioFile>, QueryError>;
    /// 根据流派查询歌曲（支持分页）
    async fn get_by_genre(
        &self,
        genre: &str,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<AudioFile>, QueryError>;
    /// 查询已收藏的音频文件列表（按用户 ID 过滤）
    async fn get_by_starred(&self, user_id: i64) -> Result<Vec<AudioFile>, QueryError>;
    /// 搜索音频文件（支持按标题、艺术家、专辑搜索）
    async fn search(
        &self,
        query: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
        title: Option<&str>,
        newer_than: Option<i64>,
        offset: i32,
        limit: i32,
    ) -> Result<(Vec<AudioFile>, i64), QueryError>;
    /// 获取播放次数最多的歌曲
    async fn get_most_played(&self, limit: i32) -> Result<Vec<AudioFile>, QueryError>;
    /// 获取最近播放的歌曲
    async fn get_recently_played(&self, limit: i32) -> Result<Vec<AudioFile>, QueryError>;
}

#[async_trait]
pub trait PlaylistDao {
    /// 根据 ID 获取播放列表（包含歌曲详情）
    async fn get_by_id(&self, id: i64) -> Result<Option<Playlist>, QueryError>;
    /// 根据所有者 ID 获取播放列表列表（基本信息）
    async fn get_by_owner_id(&self, owner_id: i64) -> Result<Vec<PlaylistSummary>, QueryError>;
}

#[async_trait]
pub trait PlayQueueDao {
    /// 根据用户 ID 获取播放队列（包含歌曲详情）
    async fn get_by_user_id(&self, user_id: i64, username: &str) -> Result<Option<PlayQueue>, QueryError>;
}

use chrono::NaiveDateTime;

/// 封面艺术信息（用于 getCoverArt API）
#[derive(Debug, Clone)]
pub struct CoverArtInfo {
    /// 封面文件路径
    pub path: String,
    /// 更新时间（用于缓存）
    pub updated_at: NaiveDateTime,
    /// 是否有嵌入封面
    pub has_embedded: bool,
}

/// 位置信息（用于查找封面）
#[derive(Debug, Clone)]
pub struct LocationInfo {
    pub protocol: String,
    pub path: String,
    pub file_count: i32,
}

/// 封面艺术路径信息（用于广度优先搜索）
#[derive(Debug, Clone)]
pub struct CoverArtPath {
    pub protocol: String,
    pub path: String,
}

/// 封面艺术路径信息（带来源类型）
#[derive(Debug, Clone)]
pub struct CoverArtPathWithSource {
    pub protocol: String,
    pub path: String,
    /// 来源类型：embedded 或 external
    pub source: String,
}

#[async_trait]
pub trait CoverArtDao {
    /// 获取音频文件的封面信息
    async fn get_audio_file_cover_info(&self, audio_file_id: i64) -> Result<Option<CoverArtInfo>, QueryError>;
    
    /// 获取专辑的封面信息（返回主要位置）
    async fn get_album_cover_info(&self, album_id: i64) -> Result<Option<CoverArtInfo>, QueryError>;
    
    /// 获取专辑的所有位置（按文件数降序）
    async fn get_album_locations(&self, album_id: i64) -> Result<Vec<LocationInfo>, QueryError>;
    
    /// 获取艺术家的封面信息
    async fn get_artist_cover_info(&self, artist_id: i64) -> Result<Option<CoverArtInfo>, QueryError>;
    
    /// 获取艺术家关联专辑的所有位置（从 album_location 查询，按路径排序，用于计算公共路径）
    async fn get_artist_album_locations(&self, artist_id: i64, limit: i32) -> Result<Vec<LocationInfo>, QueryError>;
    
    /// 获取播放列表第一首歌的封面信息
    async fn get_playlist_cover_info(&self, playlist_id: i64) -> Result<Option<CoverArtInfo>, QueryError>;
    
    /// 根据专辑 ID 获取所有封面路径（按路径排序，用于广度优先搜索）
    async fn get_cover_art_paths_by_album(&self, album_id: i64) -> Result<Vec<CoverArtPath>, QueryError>;
    
    /// 根据路径前缀获取所有封面路径（按路径排序，用于广度优先搜索）
    async fn get_cover_art_paths_by_prefix(&self, protocol: &str, path_prefix: &str) -> Result<Vec<CoverArtPath>, QueryError>;
    
    /// 根据音频文件 ID 获取关联的封面路径（带来源类型）
    async fn get_cover_art_by_audio_file(&self, audio_file_id: i64) -> Result<Option<CoverArtPathWithSource>, QueryError>;
    
    /// 获取专辑音频文件的所有封面（返回所有匹配，由应用层排序选择）
    async fn get_album_audio_file_covers(&self, album_id: i64) -> Result<Vec<CoverArtPathWithSource>, QueryError>;
    
    /// 根据艺术家 ID 获取关联的封面路径（从 cover_art 表查找 artist_id）
    async fn get_cover_art_by_artist(&self, artist_id: i64) -> Result<Option<CoverArtPath>, QueryError>;
    
    /// 获取艺术家的第一个专辑 ID（按专辑名排序）
    async fn get_first_album_id_by_artist(&self, artist_id: i64) -> Result<Option<i64>, QueryError>;
    
    /// 获取艺术家音频文件的所有封面（返回所有匹配，由应用层排序选择）
    async fn get_artist_audio_file_covers(&self, artist_id: i64) -> Result<Vec<CoverArtPathWithSource>, QueryError>;
}

/*
pub trait AlbumRepository {
    fn find_by_newest(
        &self,
        page: i64,
        per_page: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<Album>, i32), QueryError>> + Send;
    fn find_by_frequent(
        &self,
        page: i64,
        per_page: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<Album>, i32), QueryError>> + Send;
    fn find_by_random(
        &self,
        page: i64,
        per_page: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<Album>, i32), QueryError>> + Send;
    fn find_by_name(
        &self,
        page: i64,
        per_page: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<Album>, i32), QueryError>> + Send;
    fn find_by_artist(
        &self,
        page: i64,
        per_page: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<Album>, i32), QueryError>> + Send;
    fn find_by_starred(
        &self,
        page: i64,
        per_page: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<Album>, i32), QueryError>> + Send;
    fn find_by_ratting(
        &self,
        page: i64,
        per_page: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<Album>, i32), QueryError>> + Send;
    fn find_by_genre(
        &self,
        genre: String,
        page: i64,
        per_page: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<Album>, i64), QueryError>> + Send;
    fn find_by_year(
        &self,
        from_year: i32,
        to_year: i32,
        page: i64,
        per_page: i64,
    ) -> impl std::future::Future<Output = Result<Album, QueryError>> + Send;
    fn find_starred(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<Album>, QueryError>> + Send;
    fn find_by_artist_id(
        &self,
        artist_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<Album>, QueryError>> + Send;
}
    */
