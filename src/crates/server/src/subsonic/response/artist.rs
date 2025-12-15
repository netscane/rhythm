use super::album::AlbumID3;
use chrono::NaiveDateTime;
use infra::auth::JwtTokenService;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<NaiveDateTime>,

    pub user_rating: i32,

    pub cover_art: String,

    pub artist_image_url: String,
}

impl Artist {
    /// 从带 token 的 DTO 创建（应用服务层已生成 token）
    pub fn new_from_dto(
        artist_with_token: application::query::dto::artist::ArtistWithToken,
        artist_image_url: String,
    ) -> Self {
        Self {
            id: artist_with_token.artist.id.to_string(),
            name: artist_with_token.artist.name,
            starred: artist_with_token.artist.starred_at,
            user_rating: artist_with_token.artist.rating,
            cover_art: artist_with_token.cover_art_id,
            artist_image_url,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistID3 {
    pub id: String,

    pub name: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub cover_art: String,

    pub album_count: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<NaiveDateTime>,

    pub user_rating: i32,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub artist_image_url: String,

    #[serde(flatten)]
    pub os_artist_id3: Option<OpenSubsonicArtistID3>,
}

impl ArtistID3 {
    pub fn new(
        artist: model::artist::Artist,
        cover_art_token: String,
        artist_image_url: String,
    ) -> Self {
        Self {
            id: artist.id.to_string(),
            name: artist.name,
            cover_art: cover_art_token,
            album_count: artist.album_count,
            starred: artist.starred_at,
            user_rating: artist.rating,
            artist_image_url: artist_image_url,
            os_artist_id3: Some(OpenSubsonicArtistID3 {
                music_brainz_id: artist.mbz_artist_id.unwrap_or_default(),
                sort_name: artist.sort_name,
                roles: artist.roles.keys().cloned().collect(),
            }),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpenSubsonicArtistID3 {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub music_brainz_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sort_name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistWithAlbumsID3 {
    #[serde(flatten)]
    pub artist: ArtistID3,
    #[serde(rename = "album")]
    pub albums: Vec<AlbumID3>,
}

impl ArtistWithAlbumsID3 {
    pub fn new(artist: ArtistID3, albums: Vec<AlbumID3>) -> Self {
        Self { artist, albums }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistInfoBase {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub biography: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub music_brainz_id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_fm_url: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub small_image_url: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub medium_image_url: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub large_image_url: String,
}

impl ArtistInfoBase {
    pub fn new(
        artist_info: model::artist::ArtistInfo,
        small_image_url: String,
        medium_image_url: String,
        large_image_url: String,
    ) -> Self {
        Self {
            biography: artist_info.biography.unwrap_or_default(),
            music_brainz_id: "".to_string(),
            last_fm_url: "".to_string(),
            small_image_url,
            medium_image_url,
            large_image_url,
        }
    }
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistInfo {
    #[serde(flatten)]
    pub base: ArtistInfoBase,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub similar_artist: Vec<Artist>,
}

impl ArtistInfo {
    pub fn new(
        artist_info: model::artist::ArtistInfo,
        small_image_url: String,
        medium_image_url: String,
        large_image_url: String,
    ) -> Self {
        Self {
            base: ArtistInfoBase::new(
                artist_info,
                small_image_url,
                medium_image_url,
                large_image_url,
            ),
            similar_artist: vec![],
        }
    }
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistInfo2 {
    #[serde(flatten)]
    pub base: ArtistInfoBase,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub similar_artist: Vec<ArtistID3>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistID3Ref {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistsID3 {
    pub ignored_articles: String,

    pub index: Vec<Index>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    pub name: String,
    #[serde(rename = "artist")]
    pub artists: Vec<Artist>,
}
impl Index {
    /// 从带 token 的 DTO 创建（应用服务层已生成 token）
    fn new_from_dto(
        index: application::query::dto::artist::ArtistIndexWithTokens,
        base_url: &str,
    ) -> Self {
        Self {
            name: index.id,
            artists: index
                .artists
                .into_iter()
                .map(|artist_with_token| {
                    // 接口层负责 URL 生成（展示层关注点）
                    let artist_image_url = crate::subsonic::helper::image_url(
                        base_url,
                        &artist_with_token.cover_art_token,
                        300,
                    );
                    Artist::new_from_dto(artist_with_token, artist_image_url)
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Indexes {
    pub ignored_articles: String,

    pub index: Vec<Index>,
}

impl Indexes {
    pub fn new(
        ignored_articles: String,
        index: Vec<application::query::dto::artist::ArtistIndexWithTokens>,
        base_url: &str,
    ) -> Self {
        Self {
            ignored_articles,
            index: index
                .into_iter()
                .map(|index| Index::new_from_dto(index, base_url))
                .collect(),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IndexID3 {
    pub name: String,

    #[serde(rename = "artist")]
    pub artists: Vec<ArtistID3>,
}

impl IndexID3 {
    /// 从带 token 的 DTO 创建（应用服务层已生成 token）
    pub fn new_from_dto(
        index: application::query::dto::artist::ArtistIndexWithTokens,
        base_url: &str,
    ) -> Self {
        Self {
            name: index.id,
            artists: index
                .artists
                .into_iter()
                .map(|artist_with_token| {
                    // 接口层负责 URL 生成（展示层关注点）
                    let artist_image_url = crate::subsonic::helper::image_url(
                        base_url,
                        &artist_with_token.cover_art_token,
                        300,
                    );
                    ArtistID3::new(
                        artist_with_token.artist,
                        artist_with_token.cover_art_token,
                        artist_image_url,
                    )
                })
                .collect(),
        }
    }

    /// 从原始 ArtistIndex 创建（兼容旧代码，用于 get_indexes）
    pub fn new(
        index: application::query::dto::artist::ArtistIndex,
        token_service: &JwtTokenService,
        base_url: &str,
    ) -> Self {
        Self {
            name: index.id,
            artists: index
                .artists
                .into_iter()
                .map(|artist| {
                    // 为每个 artist 生成 token
                    let cover_art_id = format!("ar-{}", artist.id);
                    let cover_art_token = token_service
                        .issue_sub(&cover_art_id.clone())
                        .unwrap_or_default();
                    let artist_image_token = token_service
                        .issue_sub(artist.id.to_string().as_str())
                        .unwrap_or_default();
                    // 生成艺术家图片 URL
                    let artist_image_url =
                        crate::subsonic::helper::image_url(base_url, &artist_image_token, 300);
                    ArtistID3::new(artist, cover_art_token, artist_image_url)
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Artists {
    index: Vec<IndexID3>,
    last_modified: NaiveDateTime,
    ignored_articles: String,
}
impl Artists {
    pub fn new(
        artist_indexes: Vec<application::query::dto::artist::ArtistIndexWithTokens>,
        last_modified: NaiveDateTime,
        ignored_articles: String,
        base_url: &str,
    ) -> Self {
        Self {
            index: artist_indexes
                .into_iter()
                .map(|index| IndexID3::new_from_dto(index, base_url))
                .collect(),
            last_modified,
            ignored_articles,
        }
    }
}

/// 艺术家列表（扁平结构，不按字母分组）
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtistList {
    #[serde(rename = "artist")]
    pub artists: Vec<ArtistID3>,
}

impl ArtistList {
    pub fn new(artists: Vec<ArtistID3>) -> Self {
        Self { artists }
    }
}
