use crate::event::DomainEvent;
use crate::value::{AlbumId, ArtistId, GenreId, MediaPath, Participant};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AlbumError {
    #[error("Required parameter is missing:{0}")]
    MissingParameter(String),
    #[error("Operate type:{0} not implemented")]
    OpTypeNotImplemented(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Track not found: {0}")]
    TrackNotFound(String),
    #[error("{0}")]
    DbErr(String),
    #[error("{0}")]
    OtherErr(String),
    #[error("版本号冲突: {0}")]
    VersionConflictErr(i64),
}

#[derive(Debug, Clone)]
pub struct AlbumCreated {
    pub album_id: AlbumId,
    pub name: String,
    pub sort_name: String,
    pub genre: Option<GenreId>,
    pub genres: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct AlbumFound {
    pub album_id: AlbumId,
    pub name: String,
    pub sort_name: String,
    pub genres: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct AlbumGenreUpdated {
    pub name: String,
    pub sort_name: String,
    pub genres: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct AlbumParticipantAdded {
    pub name: String,
    pub sort_name: String,
    pub participant: Participant,
    pub all_participants: Vec<Participant>,
}
#[derive(Debug, Clone)]
pub struct AlbumParticipantRemoved {
    pub name: String,
    pub sort_name: String,
    pub participant: Participant,
    pub all_participants: Vec<Participant>,
}
#[derive(Debug, Clone)]
pub struct AlbumBoundToGenre {
    pub name: String,
    pub sort_name: String,
    pub genre_id: GenreId,
}
#[derive(Debug, Clone)]
pub struct AlbumUnboundFromGenre {
    pub name: String,
    pub sort_name: String,
    pub genre_id: GenreId,
}

#[derive(Debug, Clone)]
pub struct AlbumEvent {
    pub album_id: AlbumId,
    pub version: i64,
    pub kind: AlbumEventKind,
}

#[derive(Debug, Clone)]
pub enum AlbumEventKind {
    Created(AlbumCreated),
    Found(AlbumFound),
    ParticipantAdded(AlbumParticipantAdded),
    BoundToGenre(AlbumBoundToGenre),
    ParticipantRemoved(AlbumParticipantRemoved),
    UnboundFromGenre(AlbumUnboundFromGenre),
}

impl DomainEvent for AlbumEvent {
    fn aggregate_id(&self) -> i64 {
        self.album_id.as_i64()
    }
    fn version(&self) -> i64 {
        self.version
    }
}

#[derive(Clone)]
pub struct Album {
    /// 专辑唯一标识符
    pub id: AlbumId,

    pub name: String,

    pub sort_name: String,
    /// 专辑URI，使用第一首歌曲的URI
    pub path: MediaPath,
    /// 专辑主艺术家名称
    pub artist: Option<ArtistId>,
    // album participants (artists and other contributors)
    pub participants: Vec<Participant>,
    /// 专辑主流派
    pub genre: Option<GenreId>,
    /// 专辑所有流派列表，包含所有歌曲的流派
    pub genres: Vec<GenreId>,
    /// 专辑中歌曲的最大年份
    pub max_year: Option<i32>,
    /// 专辑中歌曲的最小年份
    pub min_year: Option<i32>,
    /// 专辑发行日期
    pub date: Option<String>,
    /// 专辑原始发行的最大年份
    pub max_original_year: Option<i32>,
    /// 专辑原始发行的最小年份
    pub min_original_year: Option<i32>,
    /// 专辑原始发行日期
    pub original_date: Option<String>,
    /// 专辑发行日期
    pub release_date: Option<String>,
    /// 发行版本数量
    pub releases: Option<i32>,
    /// 是否是合辑
    pub compilation: bool,
    /// 专辑目录编号
    pub catalog_num: Option<String>,

    pub description: Option<String>,
    /// 版本号
    pub version: i64,
    /// 待发布的事件队列
    pub pending_events: Vec<AlbumEvent>,
}

impl Album {
    // 创建新专辑
    pub fn new(id: AlbumId, name: String, sort_name: String) -> Album {
        let mut album = Album {
            id,
            name,
            sort_name,
            path: MediaPath::default(),
            artist: None,
            participants: Vec::new(),
            genre: None,
            genres: Vec::new(),
            max_year: None,
            min_year: None,
            date: None,
            max_original_year: None,
            min_original_year: None,
            original_date: None,
            release_date: None,
            releases: None,
            compilation: false,
            catalog_num: None,
            description: None,
            version: 0,
            pending_events: Vec::new(),
        };
        // emit created event
        album.pending_events.push(AlbumEvent {
            album_id: album.id.clone(),
            version: album.version,
            kind: AlbumEventKind::Created(AlbumCreated {
                album_id: album.id.clone(),
                name: album.name.clone(),
                sort_name: album.sort_name.clone(),
                genre: album.genre.clone(),
                genres: Vec::new(),
            }),
        });
        album
    }

    pub fn add_participant(&mut self, participant: Participant) -> Result<(), AlbumError> {
        if self.artist.is_none() {
            self.artist = Some(participant.artist_id.clone());
        }
        if !self.participants.iter().any(|p| p == &participant) {
            self.participants.push(participant.clone());
            self.version += 1;
            self.pending_events.push(AlbumEvent {
                album_id: self.id.clone(),
                version: self.version,
                kind: AlbumEventKind::ParticipantAdded(AlbumParticipantAdded {
                    name: self.name.clone(),
                    sort_name: self.sort_name.clone(),
                    participant: participant.clone(),
                    all_participants: self.participants.clone(),
                }),
            });
        }
        Ok(())
    }

    pub fn bind_to_genre(&mut self, genre_id: GenreId) -> Result<(), AlbumError> {
        if self.genre.is_none() {
            self.genre = Some(genre_id.clone());
        }
        if !self.genres.contains(&genre_id) {
            self.genres.push(genre_id.clone());
            self.version += 1;
            self.pending_events.push(AlbumEvent {
                album_id: self.id.clone(),
                version: self.version,
                kind: AlbumEventKind::BoundToGenre(AlbumBoundToGenre {
                    name: self.name.clone(),
                    sort_name: self.sort_name.clone(),
                    genre_id: genre_id.clone(),
                }),
            });
        }
        Ok(())
    }

    // 从事件队列中拉取所有事件
    pub fn take_events(&mut self) -> Vec<AlbumEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
#[async_trait::async_trait]
pub trait AlbumRepository: Send + Sync {
    /// 根据专辑名称和艺术家名称查找专辑
    async fn find_by_sort_name(&self, sort_name: &String) -> Result<Option<Album>, AlbumError>;

    /// 根据ID查找专辑
    async fn by_id(&self, album_id: AlbumId) -> Result<Option<Album>, AlbumError>;

    async fn save(&self, mut album: Album) -> Result<Album, AlbumError>;

    async fn delete(&self, album_id: AlbumId) -> Result<(), AlbumError>;
}
