use crate::{event::DomainEvent, value::SongIdentity};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// 歌词相关错误类型
#[derive(Error, Debug)]
pub enum LyricError {
    #[error("歌词内容不能为空")]
    EmptyContent,
    #[error("歌词格式错误: {0}")]
    InvalidFormat(String),
    #[error("时间戳格式错误: {0}")]
    InvalidTimestamp(String),
    #[error("歌词行数超出限制: {0}")]
    TooManyLines(usize),
    #[error("数据库错误: {0}")]
    DbErr(String),
    #[error("文件错误: {0}")]
    FileError(String),
    #[error("版本冲突: {0}")]
    VersionConflict(i64),
    #[error("其他错误: {0}")]
    Other(String),
}

// 歌词行结构
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricLine {
    pub timestamp: i64,       // 时间戳（毫秒）
    pub content: String,      // 歌词内容
    pub language: String,     // 语言标识
    pub is_translation: bool, // 是否为翻译
}

impl LyricLine {
    // 创建新的歌词行
    pub fn new(timestamp: i64, content: String, language: String) -> Result<Self, LyricError> {
        if content.trim().is_empty() {
            return Err(LyricError::EmptyContent);
        }
        if timestamp > 86_400_000 {
            // 24小时限制
            return Err(LyricError::InvalidTimestamp(
                "时间戳超出24小时限制".to_string(),
            ));
        }

        Ok(Self {
            timestamp,
            content: content.trim().to_string(),
            language,
            is_translation: false,
        })
    }

    // 创建翻译行
    pub fn translation(
        timestamp: i64,
        content: String,
        language: String,
    ) -> Result<Self, LyricError> {
        let mut line = Self::new(timestamp, content, language)?;
        line.is_translation = true;
        Ok(line)
    }

    // 获取格式化时间
    pub fn formatted_time(&self) -> String {
        let seconds = self.timestamp / 1000;
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{:02}:{:02}", minutes, secs)
    }
}

// 歌词元数据
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricMetadata {
    pub title: String,             // 歌词标题
    pub artist: String,            // 艺术家
    pub album: String,             // 专辑
    pub language: String,          // 主要语言
    pub source: String,            // 来源（如LRC文件、手动输入等）
    pub encoding: String,          // 编码格式
    pub created_at: NaiveDateTime, // 创建时间
    pub updated_at: NaiveDateTime, // 更新时间
    pub version: i64,              // 版本号
}

impl LyricMetadata {
    // 创建新的元数据
    pub fn new(title: String, artist: String, album: String, language: String) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            title,
            artist,
            album,
            language,
            source: "manual".to_string(),
            encoding: "UTF-8".to_string(),
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    // 更新元数据
    pub fn update(
        &mut self,
        title: Option<String>,
        artist: Option<String>,
        album: Option<String>,
        language: Option<String>,
    ) {
        if let Some(title) = title {
            self.title = title;
        }
        if let Some(artist) = artist {
            self.artist = artist;
        }
        if let Some(album) = album {
            self.album = album;
        }
        if let Some(language) = language {
            self.language = language;
        }
        self.updated_at = Utc::now().naive_utc();
        self.version += 1;
    }
}

// 歌词聚合根
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lyric {
    pub id: LyricId,                                   // 歌词ID
    pub song_id: SongIdentity,                         // 关联的歌曲ID
    pub metadata: LyricMetadata,                       // 歌词元数据
    pub lines: Vec<LyricLine>,                         // 歌词行
    pub translations: HashMap<String, Vec<LyricLine>>, // 多语言翻译
    pub status: LyricStatus,                           // 歌词状态
    pub created_at: NaiveDateTime,                     // 创建时间
    pub updated_at: NaiveDateTime,                     // 更新时间
}

impl Lyric {
    // 创建新的歌词
    pub fn new(
        song_id: SongIdentity,
        title: String,
        artist: String,
        album: String,
        language: String,
    ) -> Self {
        let now = Utc::now().naive_utc();
        let id = LyricId::new();

        Self {
            id,
            song_id,
            metadata: LyricMetadata::new(title, artist, album, language),
            lines: Vec::new(),
            translations: HashMap::new(),
            status: LyricStatus::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    // 添加歌词行
    pub fn add_line(&mut self, line: LyricLine) -> Result<(), LyricError> {
        if self.lines.len() >= 10000 {
            // 限制歌词行数
            return Err(LyricError::TooManyLines(10000));
        }

        // 检查时间戳是否已存在
        if self.lines.iter().any(|l| l.timestamp == line.timestamp) {
            return Err(LyricError::InvalidTimestamp("时间戳已存在".to_string()));
        }

        self.lines.push(line);
        self.lines.sort_by_key(|l| l.timestamp);
        self.updated_at = Utc::now().naive_utc();
        self.metadata.version += 1;

        Ok(())
    }

    // 批量添加歌词行
    pub fn add_lines(&mut self, lines: Vec<LyricLine>) -> Result<(), LyricError> {
        for line in lines {
            self.add_line(line)?;
        }
        Ok(())
    }

    // 添加翻译
    pub fn add_translation(
        &mut self,
        language: String,
        lines: Vec<LyricLine>,
    ) -> Result<(), LyricError> {
        if lines.len() != self.lines.len() {
            return Err(LyricError::InvalidFormat(
                "翻译行数必须与原文行数一致".to_string(),
            ));
        }

        // 验证时间戳一致性
        for (i, line) in lines.iter().enumerate() {
            if line.timestamp != self.lines[i].timestamp {
                return Err(LyricError::InvalidFormat(
                    "翻译时间戳与原文不一致".to_string(),
                ));
            }
        }

        self.translations.insert(language, lines);
        self.updated_at = Utc::now().naive_utc();
        self.metadata.version += 1;

        Ok(())
    }

    // 获取指定语言的歌词
    pub fn get_lyrics_by_language(&self, language: &str) -> Option<&Vec<LyricLine>> {
        if language == self.metadata.language {
            Some(&self.lines)
        } else {
            self.translations.get(language)
        }
    }

    // 获取所有支持的语言
    pub fn supported_languages(&self) -> Vec<String> {
        let mut languages = vec![self.metadata.language.clone()];
        languages.extend(self.translations.keys().cloned());
        languages
    }

    // 发布歌词
    pub fn publish(&mut self) -> Result<(), LyricError> {
        if self.lines.is_empty() {
            return Err(LyricError::EmptyContent);
        }

        self.status = LyricStatus::Published;
        self.updated_at = Utc::now().naive_utc();
        self.metadata.version += 1;

        Ok(())
    }

    // 设置为草稿状态
    pub fn set_draft(&mut self) {
        self.status = LyricStatus::Draft;
        self.updated_at = Utc::now().naive_utc();
        self.metadata.version += 1;
    }

    // 获取歌词统计信息
    pub fn statistics(&self) -> LyricStatistics {
        let total_lines = self.lines.len();
        let total_duration = if let Some(last_line) = self.lines.last() {
            last_line.timestamp
        } else {
            0
        };
        let language_count = self.supported_languages().len();

        LyricStatistics {
            total_lines,
            total_duration,
            language_count,
            is_published: self.status == LyricStatus::Published,
        }
    }

    // 验证歌词完整性
    pub fn validate(&self) -> Result<(), LyricError> {
        if self.lines.is_empty() {
            return Err(LyricError::EmptyContent);
        }

        // 检查时间戳顺序
        for i in 1..self.lines.len() {
            if self.lines[i].timestamp <= self.lines[i - 1].timestamp {
                return Err(LyricError::InvalidTimestamp("时间戳顺序错误".to_string()));
            }
        }

        // 检查翻译一致性
        for (lang, trans_lines) in &self.translations {
            if trans_lines.len() != self.lines.len() {
                return Err(LyricError::InvalidFormat(format!(
                    "{}语言翻译行数不一致",
                    lang
                )));
            }
        }

        Ok(())
    }
}

// 歌词ID值对象
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct LyricId(pub String);

impl LyricId {
    // 创建新的歌词ID
    pub fn new() -> Self {
        use uuid::Uuid;
        Self(Uuid::new_v4().to_string())
    }

    // 从字符串创建ID
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    // 获取ID字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LyricId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LyricId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// 歌词状态枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LyricStatus {
    Draft,     // 草稿
    Published, // 已发布
    Archived,  // 已归档
}

// 歌词统计信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricStatistics {
    pub total_lines: usize,    // 总行数
    pub total_duration: i64,   // 总时长（毫秒）
    pub language_count: usize, // 支持的语言数量
    pub is_published: bool,    // 是否已发布
}

// 歌词领域事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricCreated {
    pub lyric_id: LyricId,
    pub song_id: SongIdentity,
    pub created_at: NaiveDateTime,
}

impl DomainEvent for LyricCreated {
    fn event_type(&self) -> &'static str {
        "lyric.created"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricUpdated {
    pub lyric_id: LyricId,
    pub song_id: SongIdentity,
    pub updated_at: NaiveDateTime,
    pub version: i64,
}

impl DomainEvent for LyricUpdated {
    fn event_type(&self) -> &'static str {
        "lyric.updated"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricPublished {
    pub lyric_id: LyricId,
    pub song_id: SongIdentity,
    pub published_at: NaiveDateTime,
}

impl DomainEvent for LyricPublished {
    fn event_type(&self) -> &'static str {
        "lyric.published"
    }
}

// 歌词仓储接口
#[async_trait::async_trait]
pub trait LyricRepository {
    // 保存歌词
    async fn save(&self, lyric: &Lyric) -> Result<(), LyricError>;

    // 根据ID查找歌词
    async fn find_by_id(&self, id: &LyricId) -> Result<Option<Lyric>, LyricError>;

    // 根据歌曲ID查找歌词
    async fn find_by_song_id(&self, song_id: &SongIdentity) -> Result<Option<Lyric>, LyricError>;

    // 根据歌曲ID列表批量查找歌词
    async fn find_by_song_ids(&self, song_ids: &[SongIdentity]) -> Result<Vec<Lyric>, LyricError>;

    // 删除歌词
    async fn delete(&self, id: &LyricId) -> Result<(), LyricError>;

    // 检查歌词是否存在
    async fn exists(&self, id: &LyricId) -> Result<bool, LyricError>;
}

// 歌词查询服务接口
#[async_trait::async_trait]
pub trait LyricQueryService {
    // 根据歌曲ID获取歌词
    async fn get_lyrics_by_song(&self, song_id: &SongIdentity)
        -> Result<Option<Lyric>, LyricError>;

    // 根据歌曲ID列表批量获取歌词
    async fn get_lyrics_by_songs(
        &self,
        song_ids: &[SongIdentity],
    ) -> Result<Vec<Lyric>, LyricError>;

    // 搜索歌词内容
    async fn search_lyrics(
        &self,
        query: &str,
        language: Option<&str>,
    ) -> Result<Vec<Lyric>, LyricError>;

    // 获取歌词统计信息
    async fn get_lyrics_statistics(&self) -> Result<LyricStatistics, LyricError>;
}

// 歌词工厂
pub struct LyricFactory;

impl LyricFactory {
    // 从LRC文件创建歌词
    pub fn from_lrc(song_id: SongIdentity, lrc_content: &str) -> Result<Lyric, LyricError> {
        let mut lyric = Lyric::new(
            song_id,
            "Unknown Title".to_string(),
            "Unknown Artist".to_string(),
            "Unknown Album".to_string(),
            "en".to_string(),
        );

        // 解析LRC格式
        for line in lrc_content.lines() {
            if let Some(line_data) = Self::parse_lrc_line(line)? {
                lyric.add_line(line_data)?;
            }
        }

        Ok(lyric)
    }

    // 解析LRC行
    fn parse_lrc_line(line: &str) -> Result<Option<LyricLine>, LyricError> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('[') && !line.contains(']') {
            return Ok(None);
        }

        // 匹配时间戳格式 [mm:ss.xx] 或 [mm:ss:xx]
        let re = regex::Regex::new(r"\[(\d{2}):(\d{2})[.:](\d{2})\]").unwrap();

        if let Some(captures) = re.captures(line) {
            let minutes: i64 = captures[1].parse().unwrap_or(0);
            let seconds: i64 = captures[2].parse().unwrap_or(0);
            let centiseconds: i64 = captures[3].parse().unwrap_or(0);

            let timestamp = (minutes * 60 * 1000) + (seconds * 1000) + (centiseconds * 10);
            let content = line.replace(&captures[0], "").trim().to_string();

            if !content.is_empty() {
                return Ok(Some(LyricLine::new(timestamp, content, "en".to_string())?));
            }
        }

        Ok(None)
    }
}

// 歌词验证器
pub struct LyricValidator;

impl LyricValidator {
    // 验证歌词内容
    pub fn validate_content(content: &str) -> Result<(), LyricError> {
        if content.trim().is_empty() {
            return Err(LyricError::EmptyContent);
        }

        if content.len() > 1000 {
            return Err(LyricError::InvalidFormat("歌词内容过长".to_string()));
        }

        Ok(())
    }

    // 验证时间戳
    pub fn validate_timestamp(timestamp: i64) -> Result<(), LyricError> {
        if timestamp > 86_400_000 {
            // 24小时限制
            return Err(LyricError::InvalidTimestamp(
                "时间戳超出24小时限制".to_string(),
            ));
        }

        Ok(())
    }

    // 验证语言代码
    pub fn validate_language(language: &str) -> Result<(), LyricError> {
        let valid_languages = ["en", "zh", "ja", "ko", "fr", "de", "es", "it", "pt", "ru"];

        if !valid_languages.contains(&language) {
            return Err(LyricError::InvalidFormat(format!(
                "不支持的语言: {}",
                language
            )));
        }

        Ok(())
    }
}
