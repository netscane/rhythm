# 歌词聚合 (Lyric Aggregate)

## 概述

歌词聚合是音乐管理软件中的核心领域模型，负责管理歌曲的歌词信息，包括多语言支持、时间同步、版本控制等功能。

## 设计原则

本设计遵循以下DDD和CQRS原则：

1. **聚合根**: `Lyric` 作为聚合根，封装所有歌词相关的业务逻辑
2. **值对象**: `LyricId`, `LyricLine`, `LyricMetadata` 等作为不可变的值对象
3. **领域事件**: 支持歌词创建、更新、发布等事件
4. **仓储模式**: 提供读写分离的仓储接口
5. **工厂模式**: `LyricFactory` 负责复杂对象的创建
6. **验证器**: `LyricValidator` 负责业务规则验证

## 核心组件

### 1. 聚合根 (Lyric)

```rust
pub struct Lyric {
    pub id: LyricId,                      // 歌词ID
    pub song_id: SongIdentity,            // 关联的歌曲ID
    pub metadata: LyricMetadata,          // 歌词元数据
    pub lines: Vec<LyricLine>,            // 歌词行
    pub translations: HashMap<String, Vec<LyricLine>>, // 多语言翻译
    pub status: LyricStatus,              // 歌词状态
    pub created_at: NaiveDateTime,        // 创建时间
    pub updated_at: NaiveDateTime,        // 更新时间
}
```

**主要方法:**
- `new()` - 创建新歌词
- `add_line()` - 添加歌词行
- `add_translation()` - 添加翻译
- `publish()` - 发布歌词
- `validate()` - 验证歌词完整性
- `statistics()` - 获取统计信息

### 2. 值对象

#### LyricId
```rust
pub struct LyricId(pub String);
```
- 使用UUID作为唯一标识符
- 支持从字符串创建和转换

#### LyricLine
```rust
pub struct LyricLine {
    pub timestamp: u64,        // 时间戳（毫秒）
    pub content: String,       // 歌词内容
    pub language: String,      // 语言标识
    pub is_translation: bool,  // 是否为翻译
}
```
- 支持时间同步的歌词行
- 多语言支持
- 自动时间格式化

#### LyricMetadata
```rust
pub struct LyricMetadata {
    pub title: String,         // 歌词标题
    pub artist: String,        // 艺术家
    pub album: String,         // 专辑
    pub language: String,      // 主要语言
    pub source: String,        // 来源
    pub encoding: String,      // 编码格式
    pub version: i64,          // 版本号
    // ... 时间字段
}
```

### 3. 领域事件

- `LyricCreated` - 歌词创建事件
- `LyricUpdated` - 歌词更新事件
- `LyricPublished` - 歌词发布事件

### 4. 仓储接口

#### LyricRepository (写模型)
```rust
#[async_trait::async_trait]
pub trait LyricRepository {
    async fn save(&self, lyric: &Lyric) -> Result<(), LyricError>;
    async fn find_by_id(&self, id: &LyricId) -> Result<Option<Lyric>, LyricError>;
    async fn find_by_song_id(&self, song_id: &SongIdentity) -> Result<Option<Lyric>, LyricError>;
    async fn delete(&self, id: &LyricId) -> Result<(), LyricError>;
    // ... 其他方法
}
```

#### LyricQueryService (读模型)
```rust
#[async_trait::async_trait]
pub trait LyricQueryService {
    async fn get_lyrics_by_song(&self, song_id: &SongIdentity) -> Result<Option<Lyric>, LyricError>;
    async fn search_lyrics(&self, query: &str, language: Option<&str>) -> Result<Vec<Lyric>, LyricError>;
    async fn get_lyrics_statistics(&self) -> Result<LyricStatistics, LyricError>;
}
```

### 5. 工厂和验证器

#### LyricFactory
- 从LRC文件创建歌词
- 解析LRC格式的时间戳和内容
- 支持批量歌词行创建

#### LyricValidator
- 验证歌词内容长度
- 验证时间戳范围
- 验证语言代码

## 业务规则

### 1. 歌词行限制
- 单行内容不能为空
- 单行内容长度不超过1000字符
- 总行数不超过10000行
- 时间戳不能超过24小时

### 2. 时间戳规则
- 时间戳必须按升序排列
- 不能有重复的时间戳
- 翻译行的时间戳必须与原文一致

### 3. 多语言支持
- 支持10种主要语言
- 翻译行数必须与原文行数一致
- 支持原文和翻译的混合显示

### 4. 状态管理
- `Draft` - 草稿状态，可编辑
- `Published` - 已发布，只读
- `Archived` - 已归档，只读

## 使用示例

### 1. 创建歌词
```rust
let song_id = SongIdentity::new("song_123".to_string());
let mut lyric = Lyric::new(
    song_id,
    "My Song".to_string(),
    "My Artist".to_string(),
    "My Album".to_string(),
    "en".to_string(),
);
```

### 2. 添加歌词行
```rust
let line = LyricLine::new(1000, "Hello World".to_string(), "en".to_string())?;
lyric.add_line(line)?;
```

### 3. 添加翻译
```rust
let zh_lines = vec![
    LyricLine::translation(1000, "你好世界".to_string(), "zh".to_string())?,
];
lyric.add_translation("zh".to_string(), zh_lines)?;
```

### 4. 从LRC文件创建
```rust
let lrc_content = "[00:01.00]Hello\n[00:02.00]World";
let lyric = LyricFactory::from_lrc(song_id, lrc_content)?;
```

### 5. 发布歌词
```rust
lyric.publish()?;
```

## 错误处理

使用 `LyricError` 枚举处理各种错误情况：

```rust
pub enum LyricError {
    EmptyContent,                    // 歌词内容为空
    InvalidFormat(String),           // 格式错误
    InvalidTimestamp(String),        // 时间戳错误
    TooManyLines(usize),            // 行数超限
    DbErr(String),                  // 数据库错误
    FileError(String),              // 文件错误
    VersionConflict(i64),           // 版本冲突
    Other(String),                  // 其他错误
}
```

## 测试策略

- 每个公共方法都有对应的单元测试
- 使用子测试覆盖不同的分支场景
- 测试边界条件和错误情况
- 模拟外部依赖进行集成测试

## 扩展性

### 1. 新格式支持
- 可以扩展 `LyricFactory` 支持更多歌词格式
- 如KSC、TXT等格式

### 2. 新语言支持
- 在 `LyricValidator` 中添加新的语言代码
- 支持更多语言的歌词显示

### 3. 新功能支持
- 歌词评分和评论
- 歌词分享和协作
- 歌词历史版本管理

## 性能考虑

1. **批量操作**: 支持批量添加歌词行和翻译
2. **索引优化**: 为时间戳和语言建立索引
3. **缓存策略**: 对热门歌词进行缓存
4. **分页查询**: 支持大量歌词的分页显示

## 安全考虑

1. **输入验证**: 所有用户输入都经过验证
2. **权限控制**: 歌词编辑需要相应权限
3. **版本控制**: 防止并发编辑冲突
4. **数据完整性**: 确保歌词数据的一致性

## 总结

歌词聚合设计遵循DDD和CQRS原则，提供了完整的歌词管理功能。通过清晰的职责分离、完善的错误处理和全面的测试覆盖，确保了代码的可维护性和可扩展性。该设计可以很好地支持音乐管理软件中歌词相关的各种业务需求。







