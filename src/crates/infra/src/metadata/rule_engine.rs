use domain::value::{ParticipantMeta, ParticipantRole, ParticipantSubRole};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;

/// 规则执行上下文，包含原始数据和处理中的数据
#[derive(Debug, Clone)]
pub struct RuleContext {
    /// 原始标签数据
    pub raw_title: String,
    pub raw_artist: String,
    pub raw_album: String,
    pub raw_genre: String,
    pub raw_year: Option<i32>,
    pub raw_track_number: Option<i32>,

    /// 处理后的数据
    pub title: String,
    pub artists: Vec<ParticipantMeta>,
    pub album: String,
    pub genres: Vec<String>,
    pub year: Option<i32>,
    pub track_number: Option<i32>,

    /// 额外的元数据字段（用于扩展）
    pub extra: HashMap<String, String>,
}

impl RuleContext {
    pub fn new(
        title: String,
        artist: String,
        album: String,
        genre: String,
        year: Option<i32>,
        track_number: Option<i32>,
    ) -> Self {
        Self {
            raw_title: title.clone(),
            raw_artist: artist.clone(),
            raw_album: album.clone(),
            raw_genre: genre.clone(),
            raw_year: year,
            raw_track_number: track_number,
            title,
            artists: Vec::new(),
            album,
            genres: Vec::new(),
            year,
            track_number,
            extra: HashMap::new(),
        }
    }
}

/// 规则 trait，所有元数据处理规则都需要实现
pub trait MetadataRule: Send + Sync {
    /// 规则名称
    fn name(&self) -> &str;

    /// 规则优先级（数字越小优先级越高）
    fn priority(&self) -> i32 {
        100
    }

    /// 执行规则，修改上下文
    fn apply(&self, ctx: &mut RuleContext);

    /// 是否应该执行此规则
    fn should_apply(&self, ctx: &RuleContext) -> bool {
        let _ = ctx;
        true
    }
}

/// 规则引擎
pub struct MetadataRuleEngine {
    rules: Vec<Arc<dyn MetadataRule>>,
}

impl MetadataRuleEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: Arc<dyn MetadataRule>) {
        self.rules.push(rule);
    }

    /// 按优先级排序规则
    pub fn sort_rules(&mut self) {
        self.rules.sort_by_key(|r| r.priority());
    }

    /// 执行所有规则
    pub fn execute(&self, ctx: &mut RuleContext) {
        for rule in &self.rules {
            if rule.should_apply(ctx) {
                rule.apply(ctx);
            }
        }
    }

    /// 创建默认规则引擎（包含所有内置规则）
    pub fn with_default_rules() -> Self {
        let mut engine = Self::new();

        // 添加内置规则（按执行顺序）
        engine.add_rule(Arc::new(TitleCleanupRule::new()));
        engine.add_rule(Arc::new(AlbumCleanupRule::new()));       // 专辑名清理
        engine.add_rule(Arc::new(ArtistRoleExtractRule::new())); // 先提取角色标注
        engine.add_rule(Arc::new(ArtistFeatExtractRule::new())); // 提取 feat 艺术家
        engine.add_rule(Arc::new(ArtistSplitRule::new()));       // 再分割艺术家
        engine.add_rule(Arc::new(GenreSplitRule::new()));
        engine.add_rule(Arc::new(GenreNormalizeRule::new()));
        engine.add_rule(Arc::new(YearExtractRule::new()));
        engine.add_rule(Arc::new(FeatArtistExtractRule::new()));
        engine.add_rule(Arc::new(TrackNumberCleanupRule::new()));

        engine.sort_rules();
        engine
    }
}

impl Default for MetadataRuleEngine {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

// ============================================================================
// 内置规则实现
// ============================================================================

/// 标题清理规则：去除多余空格、特殊字符等
pub struct TitleCleanupRule {
    /// 需要移除的模式
    patterns_to_remove: Vec<Regex>,
}

impl TitleCleanupRule {
    pub fn new() -> Self {
        Self {
            patterns_to_remove: vec![
                // 移除文件扩展名（如果意外包含）
                Regex::new(r"\.(mp3|flac|wav|m4a|ogg|opus)$").unwrap(),
                // 移除多余空格
                Regex::new(r"\s+").unwrap(),
                // 移除开头/结尾的特殊字符
                Regex::new(r"^[\s\-_]+|[\s\-_]+$").unwrap(),
            ],
        }
    }
}

impl MetadataRule for TitleCleanupRule {
    fn name(&self) -> &str {
        "title_cleanup"
    }

    fn priority(&self) -> i32 {
        10
    }

    fn apply(&self, ctx: &mut RuleContext) {
        let mut title = ctx.title.clone();

        // 移除文件扩展名
        if let Some(re) = self.patterns_to_remove.first() {
            title = re.replace_all(&title, "").to_string();
        }

        // 规范化空格
        if let Some(re) = self.patterns_to_remove.get(1) {
            title = re.replace_all(&title, " ").to_string();
        }

        // 去除首尾特殊字符
        if let Some(re) = self.patterns_to_remove.get(2) {
            title = re.replace_all(&title, "").to_string();
        }

        ctx.title = title.trim().to_string();
    }
}

/// 专辑名清理规则：移除水印、提取 Disc 信息、清理版本标注等
pub struct AlbumCleanupRule {
    /// 网站水印模式
    watermark_patterns: Vec<Regex>,
    /// Disc 信息模式
    disc_pattern: Regex,
    /// 版本信息模式（保留在 extra 中）
    version_pattern: Regex,
}

impl AlbumCleanupRule {
    pub fn new() -> Self {
        Self {
            watermark_patterns: vec![
                // 常见音乐网站水印
                Regex::new(r"(?i)无损音乐\s*www\.[a-zA-Z0-9]+\.(net|com|cn|org)\s*").unwrap(),
                Regex::new(r"(?i)www\.[a-zA-Z0-9]+\.(net|com|cn|org)\s*").unwrap(),
                Regex::new(r"(?i)\[?无损音乐\]?\s*").unwrap(),
                Regex::new(r"(?i)@\s*\w+\s*").unwrap(), // @某某 水印
            ],
            // 匹配 [Disc 1], (Disc 2), Disc 3 等
            disc_pattern: Regex::new(r"(?i)[\[\(]?\s*Disc\s*(\d+)\s*[\]\)]?").unwrap(),
            // 匹配版本信息 [香港版], [港台版], [日本版], [Remaster] 等
            version_pattern: Regex::new(
                r"[\[\(]\s*(香港版|港台版|台湾版|日本版|韩国版|国语版|粤语版|精装版|豪华版|限量版|纪念版|Remaster|Remastered|Deluxe|Special Edition|Limited Edition)\s*[\]\)]"
            ).unwrap(),
        }
    }
}

impl MetadataRule for AlbumCleanupRule {
    fn name(&self) -> &str {
        "album_cleanup"
    }

    fn priority(&self) -> i32 {
        12 // 在标题清理之后
    }

    fn apply(&self, ctx: &mut RuleContext) {
        let mut album = ctx.album.clone();

        // 1. 移除网站水印
        for pattern in &self.watermark_patterns {
            album = pattern.replace_all(&album, "").to_string();
        }

        // 2. 提取并移除 Disc 信息
        if let Some(caps) = self.disc_pattern.captures(&album) {
            if let Some(disc_num) = caps.get(1) {
                ctx.extra
                    .insert("disc_number".to_string(), disc_num.as_str().to_string());
            }
            album = self.disc_pattern.replace_all(&album, "").to_string();
        }

        // 3. 提取并移除版本信息（保存到 extra）
        if let Some(caps) = self.version_pattern.captures(&album) {
            if let Some(version) = caps.get(1) {
                ctx.extra
                    .insert("album_version".to_string(), version.as_str().to_string());
            }
            album = self.version_pattern.replace_all(&album, "").to_string();
        }

        // 4. 清理多余空格和标点
        album = album.trim().to_string();
        // 移除末尾的冒号、破折号等
        album = Regex::new(r"[\s:：\-]+$")
            .unwrap()
            .replace_all(&album, "")
            .to_string();
        // 移除开头的冒号、破折号等
        album = Regex::new(r"^[\s:：\-]+")
            .unwrap()
            .replace_all(&album, "")
            .to_string();
        // 规范化空格
        album = Regex::new(r"\s+")
            .unwrap()
            .replace_all(&album, " ")
            .to_string();

        ctx.album = album.trim().to_string();
    }
}

/// 艺术家分割规则：将艺术家字符串按分隔符分割
pub struct ArtistSplitRule {
    /// 分隔符列表（按优先级排序）
    separators: Vec<&'static str>,
}

impl ArtistSplitRule {
    pub fn new() -> Self {
        Self {
            // 分隔符按优先级排序，先处理明确的分隔符
            separators: vec![
                // 中英文逗号
                ",",
                "，",
                // & 符号（带空格和不带空格）
                " & ",
                "&",
                // 其他分隔符
                " / ",
                "/",
                " x ",
                " X ",
                " vs ",
                " vs. ",
                " and ",
            ],
        }
    }

    pub fn with_separators(separators: Vec<&'static str>) -> Self {
        Self { separators }
    }

    fn split_artists(&self, artist_string: &str) -> Vec<String> {
        let mut result = vec![artist_string.to_string()];

        for separator in &self.separators {
            let mut new_result = Vec::new();
            for artist in result {
                if artist.contains(separator) {
                    for part in artist.split(separator) {
                        let trimmed = part.trim();
                        if !trimmed.is_empty() {
                            new_result.push(trimmed.to_string());
                        }
                    }
                } else {
                    new_result.push(artist);
                }
            }
            result = new_result;
        }

        // 去重（保持顺序，基于小写比较）
        let mut seen = std::collections::HashSet::new();
        result.retain(|x| seen.insert(x.to_lowercase()));

        result
    }
}

impl MetadataRule for ArtistSplitRule {
    fn name(&self) -> &str {
        "artist_split"
    }

    fn priority(&self) -> i32 {
        22
    }

    fn apply(&self, ctx: &mut RuleContext) {
        // 如果已经有艺术家了（可能由前置规则处理），则对每个艺术家再次分割
        if !ctx.artists.is_empty() {
            let mut new_artists = Vec::new();
            for artist in &ctx.artists {
                let split_names = self.split_artists(&artist.name);
                for name in split_names {
                    new_artists.push(ParticipantMeta {
                        role: artist.role.clone(),
                        sub_role: artist.sub_role.clone(),
                        name,
                    });
                }
            }
            // 去重
            let mut seen = std::collections::HashSet::new();
            new_artists.retain(|x| seen.insert(x.name.to_lowercase()));
            ctx.artists = new_artists;
        } else {
            // 从原始艺术家字符串分割
            let artists = self.split_artists(&ctx.raw_artist);
            ctx.artists = artists
                .into_iter()
                .map(|name| ParticipantMeta {
                    role: ParticipantRole::Artist,
                    sub_role: None,
                    name,
                })
                .collect();
        }
    }
}

/// 艺术家角色提取规则：处理 "Hanjin (Rap)" 这种带角色标注的格式
pub struct ArtistRoleExtractRule {
    /// 匹配角色标注的正则表达式
    role_pattern: Regex,
    /// 角色名称到 SubRole 的映射
    role_mappings: HashMap<String, ParticipantSubRole>,
}

impl ArtistRoleExtractRule {
    pub fn new() -> Self {
        let mut role_mappings = HashMap::new();
        // 常见角色映射
        role_mappings.insert("rap".to_string(), ParticipantSubRole::Vocals);
        role_mappings.insert("rapper".to_string(), ParticipantSubRole::Vocals);
        role_mappings.insert("vocal".to_string(), ParticipantSubRole::Vocals);
        role_mappings.insert("vocals".to_string(), ParticipantSubRole::Vocals);
        role_mappings.insert("guitar".to_string(), ParticipantSubRole::Guitar);
        role_mappings.insert("bass".to_string(), ParticipantSubRole::Bass);
        role_mappings.insert("drums".to_string(), ParticipantSubRole::Drums);
        role_mappings.insert("keyboard".to_string(), ParticipantSubRole::Keyboard);
        role_mappings.insert("piano".to_string(), ParticipantSubRole::Keyboard);
        role_mappings.insert("keys".to_string(), ParticipantSubRole::Keyboard);

        Self {
            // 匹配 "Name (Role)" 或 "Name [Role]" 格式
            role_pattern: Regex::new(r"^(.+?)\s*[\(\[]([\w\s]+)[\)\]]$").unwrap(),
            role_mappings,
        }
    }

    fn extract_role(&self, artist_str: &str) -> (String, Option<ParticipantSubRole>) {
        if let Some(caps) = self.role_pattern.captures(artist_str) {
            let name = caps.get(1).map(|m| m.as_str().trim()).unwrap_or(artist_str);
            let role_str = caps.get(2).map(|m| m.as_str().trim().to_lowercase());

            if let Some(role_key) = role_str {
                let sub_role = self.role_mappings.get(&role_key).cloned();
                return (name.to_string(), sub_role);
            }
        }
        (artist_str.to_string(), None)
    }
}

impl MetadataRule for ArtistRoleExtractRule {
    fn name(&self) -> &str {
        "artist_role_extract"
    }

    fn priority(&self) -> i32 {
        18 // 在分割之前执行
    }

    fn apply(&self, ctx: &mut RuleContext) {
        // 处理原始艺术家字符串，先按逗号分割，然后提取角色
        let parts: Vec<&str> = ctx.raw_artist.split(&[',', '，'][..]).collect();
        let mut artists = Vec::new();

        for part in parts {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }

            let (name, sub_role) = self.extract_role(trimmed);
            artists.push(ParticipantMeta {
                role: ParticipantRole::Artist,
                sub_role,
                name,
            });
        }

        if !artists.is_empty() {
            ctx.artists = artists;
        }
    }
}

/// 艺术家 Feat 提取规则：处理 "李宗盛 (Feat. 李剑青 白安)" 这种格式
pub struct ArtistFeatExtractRule {
    /// 匹配 feat 模式的正则表达式
    feat_patterns: Vec<Regex>,
}

impl ArtistFeatExtractRule {
    pub fn new() -> Self {
        Self {
            feat_patterns: vec![
                // (Feat. xxx xxx) 或 (feat. xxx xxx)
                Regex::new(r"(?i)^(.+?)\s*\(\s*feat\.?\s+(.+?)\s*\)$").unwrap(),
                // [Feat. xxx xxx]
                Regex::new(r"(?i)^(.+?)\s*\[\s*feat\.?\s+(.+?)\s*\]$").unwrap(),
                // (Ft. xxx xxx)
                Regex::new(r"(?i)^(.+?)\s*\(\s*ft\.?\s+(.+?)\s*\)$").unwrap(),
                // (featuring xxx xxx)
                Regex::new(r"(?i)^(.+?)\s*\(\s*featuring\s+(.+?)\s*\)$").unwrap(),
            ],
        }
    }

    fn split_feat_artists(&self, feat_str: &str) -> Vec<String> {
        // feat 艺术家可能用空格分隔（中文名）或逗号/&分隔
        let mut result = Vec::new();

        // 先尝试按逗号和&分割
        let has_separator = feat_str.contains(',')
            || feat_str.contains('，')
            || feat_str.contains('&')
            || feat_str.contains('/');

        if has_separator {
            for sep in &[",", "，", "&", "/"] {
                if feat_str.contains(sep) {
                    for part in feat_str.split(sep) {
                        let trimmed = part.trim();
                        if !trimmed.is_empty() {
                            result.push(trimmed.to_string());
                        }
                    }
                    return result;
                }
            }
        }

        // 如果没有明确分隔符，按空格分割（适用于中文名）
        // 但要注意英文名可能包含空格，所以只对纯中文字符串这样处理
        let is_mostly_chinese = feat_str
            .chars()
            .filter(|c| !c.is_whitespace())
            .filter(|c| *c >= '\u{4e00}' && *c <= '\u{9fff}')
            .count()
            > feat_str.chars().filter(|c| c.is_ascii_alphabetic()).count();

        if is_mostly_chinese {
            for part in feat_str.split_whitespace() {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    result.push(trimmed.to_string());
                }
            }
        } else {
            // 英文名，整体作为一个艺术家
            result.push(feat_str.to_string());
        }

        result
    }
}

impl MetadataRule for ArtistFeatExtractRule {
    fn name(&self) -> &str {
        "artist_feat_extract"
    }

    fn priority(&self) -> i32 {
        19 // 在角色提取之后，分割之前
    }

    fn apply(&self, ctx: &mut RuleContext) {
        // 检查原始艺术家字符串是否包含 feat 模式
        for pattern in &self.feat_patterns {
            if let Some(caps) = pattern.captures(&ctx.raw_artist) {
                let main_artist = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                let feat_artists_str = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");

                if !main_artist.is_empty() {
                    ctx.artists.clear();

                    // 添加主艺术家
                    ctx.artists.push(ParticipantMeta {
                        role: ParticipantRole::Artist,
                        sub_role: None,
                        name: main_artist.to_string(),
                    });

                    // 分割并添加 feat 艺术家
                    let feat_artists = self.split_feat_artists(feat_artists_str);
                    for name in feat_artists {
                        ctx.artists.push(ParticipantMeta {
                            role: ParticipantRole::Artist,
                            sub_role: None,
                            name,
                        });
                    }

                    // 更新 raw_artist 以便后续规则处理
                    ctx.raw_artist = main_artist.to_string();
                    return;
                }
            }
        }
    }
}

/// 流派分割规则
pub struct GenreSplitRule {
    separators: Vec<char>,
}

impl GenreSplitRule {
    pub fn new() -> Self {
        Self {
            separators: vec![',', ';', '/', '|'],
        }
    }
}

impl MetadataRule for GenreSplitRule {
    fn name(&self) -> &str {
        "genre_split"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn apply(&self, ctx: &mut RuleContext) {
        let mut genres: Vec<String> = vec![ctx.raw_genre.clone()];

        for sep in &self.separators {
            let mut new_genres = Vec::new();
            for genre in genres {
                for part in genre.split(*sep) {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        new_genres.push(trimmed.to_string());
                    }
                }
            }
            genres = new_genres;
        }

        ctx.genres = genres;
    }
}

/// 流派规范化规则：统一流派名称
pub struct GenreNormalizeRule {
    mappings: HashMap<String, String>,
}

impl GenreNormalizeRule {
    pub fn new() -> Self {
        let mut mappings = HashMap::new();

        // 常见流派别名映射
        mappings.insert("hiphop".to_string(), "Hip-Hop".to_string());
        mappings.insert("hip hop".to_string(), "Hip-Hop".to_string());
        mappings.insert("hip-hop".to_string(), "Hip-Hop".to_string());
        mappings.insert("r&b".to_string(), "R&B".to_string());
        mappings.insert("rnb".to_string(), "R&B".to_string());
        mappings.insert("rhythm and blues".to_string(), "R&B".to_string());
        mappings.insert("electronica".to_string(), "Electronic".to_string());
        mappings.insert("electro".to_string(), "Electronic".to_string());
        mappings.insert("rock n roll".to_string(), "Rock".to_string());
        mappings.insert("rock & roll".to_string(), "Rock".to_string());
        mappings.insert("rock'n'roll".to_string(), "Rock".to_string());
        mappings.insert("heavy metal".to_string(), "Metal".to_string());
        mappings.insert("death metal".to_string(), "Metal".to_string());
        mappings.insert("black metal".to_string(), "Metal".to_string());
        mappings.insert("post-rock".to_string(), "Post-Rock".to_string());
        mappings.insert("postrock".to_string(), "Post-Rock".to_string());
        mappings.insert("indie rock".to_string(), "Indie".to_string());
        mappings.insert("indie pop".to_string(), "Indie".to_string());
        mappings.insert("alt rock".to_string(), "Alternative".to_string());
        mappings.insert("alternative rock".to_string(), "Alternative".to_string());
        mappings.insert("country & western".to_string(), "Country".to_string());
        mappings.insert("c&w".to_string(), "Country".to_string());
        mappings.insert("drum and bass".to_string(), "Drum & Bass".to_string());
        mappings.insert("dnb".to_string(), "Drum & Bass".to_string());
        mappings.insert("d&b".to_string(), "Drum & Bass".to_string());
        mappings.insert("trip hop".to_string(), "Trip-Hop".to_string());
        mappings.insert("triphop".to_string(), "Trip-Hop".to_string());

        Self { mappings }
    }

    pub fn with_mappings(mappings: HashMap<String, String>) -> Self {
        Self { mappings }
    }
}

impl MetadataRule for GenreNormalizeRule {
    fn name(&self) -> &str {
        "genre_normalize"
    }

    fn priority(&self) -> i32 {
        35
    }

    fn apply(&self, ctx: &mut RuleContext) {
        ctx.genres = ctx
            .genres
            .iter()
            .map(|g| {
                let lower = g.to_lowercase();
                self.mappings
                    .get(&lower)
                    .cloned()
                    .unwrap_or_else(|| {
                        // 如果没有映射，首字母大写
                        if g.is_empty() {
                            "[Unknown]".to_string()
                        } else {
                            let mut chars = g.chars();
                            match chars.next() {
                                None => g.clone(),
                                Some(first) => {
                                    first.to_uppercase().collect::<String>() + chars.as_str()
                                }
                            }
                        }
                    })
            })
            .collect();
    }
}

/// 从标题中提取 feat 艺术家的规则
pub struct FeatArtistExtractRule {
    patterns: Vec<Regex>,
}

impl FeatArtistExtractRule {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                // (feat. Artist Name)
                Regex::new(r"(?i)\s*\(feat\.?\s+([^)]+)\)").unwrap(),
                // [feat. Artist Name]
                Regex::new(r"(?i)\s*\[feat\.?\s+([^\]]+)\]").unwrap(),
                // feat. Artist Name (at end)
                Regex::new(r"(?i)\s+feat\.?\s+(.+)$").unwrap(),
                // (ft. Artist Name)
                Regex::new(r"(?i)\s*\(ft\.?\s+([^)]+)\)").unwrap(),
                // ft. Artist Name (at end)
                Regex::new(r"(?i)\s+ft\.?\s+(.+)$").unwrap(),
                // (featuring Artist Name)
                Regex::new(r"(?i)\s*\(featuring\s+([^)]+)\)").unwrap(),
                // featuring Artist Name (at end)
                Regex::new(r"(?i)\s+featuring\s+(.+)$").unwrap(),
            ],
        }
    }
}

impl MetadataRule for FeatArtistExtractRule {
    fn name(&self) -> &str {
        "feat_artist_extract"
    }

    fn priority(&self) -> i32 {
        25
    }

    fn apply(&self, ctx: &mut RuleContext) {
        let mut title = ctx.title.clone();

        for pattern in &self.patterns {
            if let Some(caps) = pattern.captures(&title) {
                if let Some(artist_match) = caps.get(1) {
                    let feat_artist = artist_match.as_str().trim();

                    // 添加 feat 艺术家（如果不存在）
                    let exists = ctx
                        .artists
                        .iter()
                        .any(|a| a.name.to_lowercase() == feat_artist.to_lowercase());

                    if !exists && !feat_artist.is_empty() {
                        ctx.artists.push(ParticipantMeta {
                            role: ParticipantRole::Artist,
                            sub_role: None,
                            name: feat_artist.to_string(),
                        });
                    }

                    // 从标题中移除 feat 部分
                    title = pattern.replace(&title, "").to_string();
                }
            }
        }

        ctx.title = title.trim().to_string();
    }
}

/// 从专辑名或标题中提取年份的规则
pub struct YearExtractRule {
    pattern: Regex,
}

impl YearExtractRule {
    pub fn new() -> Self {
        Self {
            // 匹配 (1999), [2020], 1990-2000 等模式
            pattern: Regex::new(r"\b(19[0-9]{2}|20[0-2][0-9])\b").unwrap(),
        }
    }
}

impl MetadataRule for YearExtractRule {
    fn name(&self) -> &str {
        "year_extract"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn should_apply(&self, ctx: &RuleContext) -> bool {
        // 只有当年份为空时才尝试提取
        ctx.year.is_none()
    }

    fn apply(&self, ctx: &mut RuleContext) {
        // 优先从专辑名提取
        if let Some(caps) = self.pattern.captures(&ctx.album) {
            if let Some(year_match) = caps.get(1) {
                if let Ok(year) = year_match.as_str().parse::<i32>() {
                    ctx.year = Some(year);
                    return;
                }
            }
        }

        // 其次从标题提取
        if let Some(caps) = self.pattern.captures(&ctx.title) {
            if let Some(year_match) = caps.get(1) {
                if let Ok(year) = year_match.as_str().parse::<i32>() {
                    ctx.year = Some(year);
                }
            }
        }
    }
}

/// 音轨号清理规则
pub struct TrackNumberCleanupRule;

impl TrackNumberCleanupRule {
    pub fn new() -> Self {
        Self
    }
}

impl MetadataRule for TrackNumberCleanupRule {
    fn name(&self) -> &str {
        "track_number_cleanup"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn apply(&self, ctx: &mut RuleContext) {
        // 确保音轨号有效
        if let Some(track) = ctx.track_number {
            if track <= 0 {
                ctx.track_number = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artist_split_comma_separated() {
        // 测试: 梁洛施,Boy'z,关智斌,郑希怡,Hanjin (Rap)
        let engine = MetadataRuleEngine::with_default_rules();
        let mut ctx = RuleContext::new(
            "Test Song".to_string(),
            "梁洛施,Boy'z,关智斌,郑希怡,Hanjin (Rap)".to_string(),
            "Test Album".to_string(),
            "Pop".to_string(),
            Some(2020),
            Some(1),
        );

        engine.execute(&mut ctx);

        println!("Artists: {:?}", ctx.artists);
        assert_eq!(ctx.artists.len(), 5);
        assert_eq!(ctx.artists[0].name, "梁洛施");
        assert_eq!(ctx.artists[1].name, "Boy'z");
        assert_eq!(ctx.artists[2].name, "关智斌");
        assert_eq!(ctx.artists[3].name, "郑希怡");
        assert_eq!(ctx.artists[4].name, "Hanjin");
        // Hanjin 应该有 Vocals sub_role (Rap 映射)
        assert_eq!(ctx.artists[4].sub_role, Some(ParticipantSubRole::Vocals));
    }

    #[test]
    fn test_artist_split_ampersand_separated() {
        // 测试: 好妹妹&秦昊&张小厚&陈粒&粒粒&焦迈奇&王加一&陈婧霏
        let engine = MetadataRuleEngine::with_default_rules();
        let mut ctx = RuleContext::new(
            "Test Song".to_string(),
            "好妹妹&秦昊&张小厚&陈粒&粒粒&焦迈奇&王加一&陈婧霏".to_string(),
            "Test Album".to_string(),
            "Pop".to_string(),
            None,
            None,
        );

        engine.execute(&mut ctx);

        println!("Artists: {:?}", ctx.artists);
        assert_eq!(ctx.artists.len(), 8);
        assert_eq!(ctx.artists[0].name, "好妹妹");
        assert_eq!(ctx.artists[1].name, "秦昊");
        assert_eq!(ctx.artists[7].name, "陈婧霏");
    }

    #[test]
    fn test_artist_feat_with_space_separated_chinese() {
        // 测试: 李宗盛 (Feat. 李剑青 白安)
        let engine = MetadataRuleEngine::with_default_rules();
        let mut ctx = RuleContext::new(
            "Test Song".to_string(),
            "李宗盛 (Feat. 李剑青 白安)".to_string(),
            "Test Album".to_string(),
            "Pop".to_string(),
            None,
            None,
        );

        engine.execute(&mut ctx);

        println!("Artists: {:?}", ctx.artists);
        assert_eq!(ctx.artists.len(), 3);
        assert_eq!(ctx.artists[0].name, "李宗盛");
        assert_eq!(ctx.artists[1].name, "李剑青");
        assert_eq!(ctx.artists[2].name, "白安");
    }

    #[test]
    fn test_artist_split_rule() {
        let rule = ArtistSplitRule::new();
        let mut ctx = RuleContext::new(
            "Test Song".to_string(),
            "Artist A & Artist B".to_string(),
            "Test Album".to_string(),
            "Pop".to_string(),
            Some(2020),
            Some(1),
        );

        rule.apply(&mut ctx);

        assert_eq!(ctx.artists.len(), 2);
        assert_eq!(ctx.artists[0].name, "Artist A");
        assert_eq!(ctx.artists[1].name, "Artist B");
    }

    #[test]
    fn test_feat_artist_extract_rule() {
        let rule = FeatArtistExtractRule::new();
        let mut ctx = RuleContext::new(
            "Song Title (feat. Guest Artist)".to_string(),
            "Main Artist".to_string(),
            "Album".to_string(),
            "Pop".to_string(),
            None,
            None,
        );

        // 先添加主艺术家
        ctx.artists.push(ParticipantMeta {
            role: ParticipantRole::Artist,
            sub_role: None,
            name: "Main Artist".to_string(),
        });

        rule.apply(&mut ctx);

        assert_eq!(ctx.title, "Song Title");
        assert_eq!(ctx.artists.len(), 2);
        assert_eq!(ctx.artists[1].name, "Guest Artist");
    }

    #[test]
    fn test_genre_normalize_rule() {
        let rule = GenreNormalizeRule::new();
        let mut ctx = RuleContext::new(
            "Test".to_string(),
            "Artist".to_string(),
            "Album".to_string(),
            "hiphop".to_string(),
            None,
            None,
        );
        ctx.genres = vec!["hiphop".to_string(), "r&b".to_string()];

        rule.apply(&mut ctx);

        assert_eq!(ctx.genres[0], "Hip-Hop");
        assert_eq!(ctx.genres[1], "R&B");
    }

    #[test]
    fn test_year_extract_rule() {
        let rule = YearExtractRule::new();
        let mut ctx = RuleContext::new(
            "Song".to_string(),
            "Artist".to_string(),
            "Album (2019 Remaster)".to_string(),
            "Rock".to_string(),
            None,
            None,
        );

        rule.apply(&mut ctx);

        assert_eq!(ctx.year, Some(2019));
    }

    #[test]
    fn test_full_engine() {
        let engine = MetadataRuleEngine::with_default_rules();
        let mut ctx = RuleContext::new(
            "Awesome Song (feat. Guest)".to_string(),
            "Main Artist & Second Artist".to_string(),
            "Great Album".to_string(),
            "hiphop, r&b".to_string(),
            None,
            Some(5),
        );

        engine.execute(&mut ctx);

        assert_eq!(ctx.title, "Awesome Song");
        assert!(ctx.artists.len() >= 2);
        assert!(ctx.genres.contains(&"Hip-Hop".to_string()));
    }

    #[test]
    fn test_role_extract() {
        let rule = ArtistRoleExtractRule::new();
        let mut ctx = RuleContext::new(
            "Test".to_string(),
            "Artist Name (Rap)".to_string(),
            "Album".to_string(),
            "Pop".to_string(),
            None,
            None,
        );

        rule.apply(&mut ctx);

        assert_eq!(ctx.artists.len(), 1);
        assert_eq!(ctx.artists[0].name, "Artist Name");
        assert_eq!(ctx.artists[0].sub_role, Some(ParticipantSubRole::Vocals));
    }

    #[test]
    fn test_album_cleanup_disc_info() {
        let engine = MetadataRuleEngine::with_default_rules();

        // 测试: 愛上原味: 萬芳 [Disc 1]
        let mut ctx = RuleContext::new(
            "Test".to_string(),
            "Artist".to_string(),
            "愛上原味: 萬芳 [Disc 1]".to_string(),
            "Pop".to_string(),
            None,
            None,
        );
        engine.execute(&mut ctx);
        println!("Album: '{}', Extra: {:?}", ctx.album, ctx.extra);
        assert_eq!(ctx.album, "愛上原味: 萬芳");
        assert_eq!(ctx.extra.get("disc_number"), Some(&"1".to_string()));
    }

    #[test]
    fn test_album_cleanup_version_info() {
        let engine = MetadataRuleEngine::with_default_rules();

        // 测试: 茹此精彩十三首 [香港版]
        let mut ctx = RuleContext::new(
            "Test".to_string(),
            "Artist".to_string(),
            "茹此精彩十三首 [香港版]".to_string(),
            "Pop".to_string(),
            None,
            None,
        );
        engine.execute(&mut ctx);
        println!("Album: '{}', Extra: {:?}", ctx.album, ctx.extra);
        assert_eq!(ctx.album, "茹此精彩十三首");
        assert_eq!(ctx.extra.get("album_version"), Some(&"香港版".to_string()));
    }

    #[test]
    fn test_album_cleanup_disc_and_version() {
        let engine = MetadataRuleEngine::with_default_rules();

        // 测试: 一人一首成名曲[港台版] [Disc 1]
        let mut ctx = RuleContext::new(
            "Test".to_string(),
            "Artist".to_string(),
            "一人一首成名曲[港台版] [Disc 1]".to_string(),
            "Pop".to_string(),
            None,
            None,
        );
        engine.execute(&mut ctx);
        println!("Album: '{}', Extra: {:?}", ctx.album, ctx.extra);
        assert_eq!(ctx.album, "一人一首成名曲");
        assert_eq!(ctx.extra.get("disc_number"), Some(&"1".to_string()));
        assert_eq!(ctx.extra.get("album_version"), Some(&"港台版".to_string()));
    }

    #[test]
    fn test_album_cleanup_watermark() {
        let engine = MetadataRuleEngine::with_default_rules();

        // 测试: 无损音乐www.23ape.net  邓丽君24K金藏集 (Disc 3)
        let mut ctx = RuleContext::new(
            "Test".to_string(),
            "Artist".to_string(),
            "无损音乐www.23ape.net  邓丽君24K金藏集 (Disc 3)".to_string(),
            "Pop".to_string(),
            None,
            None,
        );
        engine.execute(&mut ctx);
        println!("Album: '{}', Extra: {:?}", ctx.album, ctx.extra);
        assert_eq!(ctx.album, "邓丽君24K金藏集");
        assert_eq!(ctx.extra.get("disc_number"), Some(&"3".to_string()));
    }

    #[test]
    fn test_album_cleanup_multiple_disc_formats() {
        let engine = MetadataRuleEngine::with_default_rules();

        // 测试: 经典名曲 [Disc 2]
        let mut ctx = RuleContext::new(
            "Test".to_string(),
            "Artist".to_string(),
            "经典名曲 [Disc 2]".to_string(),
            "Pop".to_string(),
            None,
            None,
        );
        engine.execute(&mut ctx);
        println!("Album: '{}', Extra: {:?}", ctx.album, ctx.extra);
        assert_eq!(ctx.album, "经典名曲");
        assert_eq!(ctx.extra.get("disc_number"), Some(&"2".to_string()));
    }
}
