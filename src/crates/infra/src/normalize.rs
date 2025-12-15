use unidecode::unidecode;

use application::command::album::AlbumNameNormalizer;
use application::command::artist::ArtistNameNormalizer;
use once_cell::sync::Lazy;
use std::collections::HashMap;

static UTF8_TO_ASCII: Lazy<HashMap<char, char>> = Lazy::new(|| {
    let mut map = HashMap::new();
    // 单引号替换
    for c in "'\'‛′".chars() {
        map.insert(c, '\'');
    }
    // 双引号替换
    for c in "＂〃ˮײ᳓″‶˶ʺ\"\"˝‟".chars() {
        map.insert(c, '"');
    }
    // 连字符替换
    for c in "‐–—−―".chars() {
        map.insert(c, '-');
    }
    map
});

/// 清理字符串，将特殊的 UTF-8 字符替换为对应的 ASCII 字符
pub fn clear(name: &str) -> String {
    name.chars()
        .map(|c| UTF8_TO_ASCII.get(&c).copied().unwrap_or(c))
        .collect()
}

/// 移除字符串开头的文章词
pub fn remove_article(name: &str, articles: &[String]) -> String {
    for article in articles {
        let prefix = format!("{} ", article);
        if name.starts_with(&prefix) {
            return name[prefix.len()..].to_string();
        }
    }
    name.to_string()
}

/// 清理字符串用于排序，移除重音符号、文章词，并转换为小写
pub fn sanitize_no_article(original_value: &str, articles: &[String]) -> String {
    // 1. 移除重音符号
    let without_accents = unidecode(original_value);
    // 2. 移除文章词
    let without_article = remove_article(without_accents.trim(), articles);
    // 3. 清理特殊字符并转换为小写
    clear(without_article.trim().to_lowercase().as_str())
}

pub struct ArtistNameNormalizerImpl {
    ignored_articles: Vec<String>,
}

impl ArtistNameNormalizerImpl {
    pub fn new(ignored_articles: &[String]) -> Self {
        Self {
            ignored_articles: ignored_articles.to_vec(),
        }
    }
}
impl ArtistNameNormalizer for ArtistNameNormalizerImpl {
    fn normalize(&self, name: &String) -> String {
        sanitize_no_article(name, &self.ignored_articles)
    }
}

pub struct AlbumNameNormalizerImpl {
    ignored_articles: Vec<String>,
}

impl AlbumNameNormalizerImpl {
    pub fn new(ignored_articles: &[String]) -> Self {
        Self {
            ignored_articles: ignored_articles.to_vec(),
        }
    }
}
impl AlbumNameNormalizer for AlbumNameNormalizerImpl {
    fn normalize(&self, name: &String) -> String {
        sanitize_no_article(name, &self.ignored_articles)
    }
}
