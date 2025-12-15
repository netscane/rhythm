use super::dao::ArtistDao;
use super::dto::artist::{ArtistIndex, ArtistIndexWithTokens, ArtistWithToken};
use super::dto::cover_art;
use super::shared::CoverArtTokenService;
use super::QueryError;
use lazy_static::lazy_static;
use log::info;
use model::artist::Artist;
use pinyin::ToPinyin;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;

type IndexGroups = HashMap<String, String>;

lazy_static! {
    static ref INDEX_GROUPS_RX: Regex = Regex::new(r"(.+)\((.+)\)").unwrap();
}

pub struct ArtistIndexRule {
    index_groups: IndexGroups,
    prefer_sort_tags: bool,
}

pub struct ArtistService<T>
where
    T: ArtistDao + Send + Sync,
{
    artist_dao: Arc<T>,
    index_rule: ArtistIndexRule,
    token_service: Option<Arc<dyn CoverArtTokenService>>,
}

impl ArtistIndexRule {
    fn parse_index_groups(specs: &str) -> IndexGroups {
        let mut parsed = HashMap::new();
        for g in specs.split_whitespace() {
            if let Some(caps) = INDEX_GROUPS_RX.captures(g) {
                let group = caps.get(1).unwrap().as_str();
                let chars = caps.get(2).unwrap().as_str();
                for c in chars.chars() {
                    parsed.insert(c.to_string(), group.to_string());
                }
            } else {
                // 如果正则表达式不匹配，将该组作为键和值都添加
                parsed.insert(g.to_string(), g.to_string());
            }
        }
        parsed
    }
    pub fn new(specs: &str, prefer_sort_tags: bool) -> Self {
        Self {
            index_groups: Self::parse_index_groups(specs),
            prefer_sort_tags,
        }
    }
}

impl<T> ArtistService<T>
where
    T: ArtistDao + Send + Sync,
{
    pub fn new(artist_dao: Arc<T>, index_rule: ArtistIndexRule) -> Self {
        Self {
            artist_dao,
            index_rule,
            token_service: None,
        }
    }

    pub fn with_token_service(
        artist_dao: Arc<T>,
        index_rule: ArtistIndexRule,
        token_service: Arc<dyn CoverArtTokenService>,
    ) -> Self {
        Self {
            artist_dao,
            index_rule,
            token_service: Some(token_service),
        }
    }
    pub async fn get_indexes(&self) -> Result<Vec<ArtistIndex>, QueryError> {
        let dao = self.artist_dao.clone();
        let artists = dao.get_all().await?;
        // group by index key
        let mut index = HashMap::new();
        for artist in artists {
            let key = self.get_index_key(&artist);
            index.entry(key).or_insert_with(Vec::new).push(artist);
        }

        // convert to Vec and sort
        let mut result: Vec<ArtistIndex> = index
            .into_iter()
            .map(|(id, artists)| ArtistIndex { id, artists })
            .collect();

        result.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(result)
    }

    /// 获取索引列表（带 token）
    pub async fn get_indexes_with_tokens(&self) -> Result<Vec<ArtistIndexWithTokens>, QueryError> {
        let token_service = self
            .token_service
            .as_ref()
            .ok_or_else(|| QueryError::InvalidInput("Token service not available".to_string()))?;

        let dao = self.artist_dao.clone();
        let artists = dao.get_all().await?;

        // group by index key
        let mut index = HashMap::new();
        for artist in artists {
            let key = self.get_index_key(&artist);
            index.entry(key).or_insert_with(Vec::new).push(artist);
        }

        // convert to Vec with tokens and sort
        let mut result: Vec<ArtistIndexWithTokens> = index
            .into_iter()
            .map(|(id, artists)| {
                let artists_with_tokens: Vec<ArtistWithToken> = artists
                    .into_iter()
                    .map(|artist| {
                        let cover_art_id = cover_art::artist_cover_art_id(artist.id);
                        let cover_art_token = token_service
                            .issue_cover_art_token(cover_art_id.clone())
                            .unwrap_or_default();
                        ArtistWithToken {
                            artist,
                            cover_art_id,
                            cover_art_token,
                        }
                    })
                    .collect();
                ArtistIndexWithTokens {
                    id,
                    artists: artists_with_tokens,
                }
            })
            .collect();

        result.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(result)
    }
    pub async fn get_artists(&self) -> Result<Vec<ArtistIndex>, QueryError> {
        info!("xx get_artists");
        let dao = self.artist_dao.clone();
        let artists = dao.get_all().await?;
        info!("xx get_artists artists: {}", artists.len());
        // group by index key
        let mut index = HashMap::new();
        for artist in artists {
            let key = self.get_index_key(&artist);
            index.entry(key).or_insert_with(Vec::new).push(artist);
        }

        // convert to Vec and sort
        let mut result: Vec<ArtistIndex> = index
            .into_iter()
            .map(|(id, artists)| ArtistIndex { id, artists })
            .collect();

        result.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(result)
    }

    /// 获取艺术家列表（带 token）
    pub async fn get_artists_with_tokens(&self) -> Result<Vec<ArtistIndexWithTokens>, QueryError> {
        let token_service = self
            .token_service
            .as_ref()
            .ok_or_else(|| QueryError::InvalidInput("Token service not available".to_string()))?;

        let dao = self.artist_dao.clone();
        let artists = dao.get_all().await?;

        // group by index key
        let mut index = HashMap::new();
        for artist in artists {
            let key = self.get_index_key(&artist);
            index.entry(key).or_insert_with(Vec::new).push(artist);
        }

        // convert to Vec with tokens and sort
        let mut result: Vec<ArtistIndexWithTokens> = index
            .into_iter()
            .map(|(id, artists)| {
                let artists_with_tokens: Vec<ArtistWithToken> = artists
                    .into_iter()
                    .map(|artist| {
                        let artist_cover_id = cover_art::artist_cover_art_id(artist.id);
                        let cover_art_token = token_service
                            .issue_cover_art_token(artist_cover_id.clone())
                            .unwrap_or_default();
                        ArtistWithToken {
                            artist,
                            cover_art_id: artist_cover_id,
                            cover_art_token,
                        }
                    })
                    .collect();
                ArtistIndexWithTokens {
                    id,
                    artists: artists_with_tokens,
                }
            })
            .collect();

        result.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(result)
    }
    fn get_index_key(&self, artist: &Artist) -> String {
        let source = if self.index_rule.prefer_sort_tags && artist.sort_name.len() > 0 {
            &artist.sort_name
        } else {
            &artist.order_name
        };

        let name = source.to_lowercase();

        // 先检查非中文部分
        for (k, v) in &self.index_rule.index_groups {
            if name.starts_with(&k.to_lowercase()) {
                return v.clone();
            }
        }

        // 如果没有匹配的非中文索引，再检查中文部分
        if name.chars().any(|c| c >= '\u{4e00}' && c <= '\u{9fff}') {
            if let Some(first_char) = name.chars().next() {
                if let Some(pinyin) = first_char.to_pinyin() {
                    // 使用不带声调的拼音，取首字母
                    let pinyin_str = pinyin.plain();
                    if let Some(first_pinyin_char) = pinyin_str.chars().next() {
                        let result = first_pinyin_char.to_uppercase().to_string();
                        return result;
                    }
                }
            }
        }

        "#".to_string()
    }
}
