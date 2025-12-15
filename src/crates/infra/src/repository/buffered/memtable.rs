use async_trait::async_trait;
use log::{error, info, warn};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::{oneshot, RwLock};

// 旋转原因枚举
#[derive(Debug, Clone, Copy)]
pub enum RotateReason {
    SizeThreshold, // 达到大小阈值
    Timeout,       // 超时触发
    Shutdown,      // 优雅关闭
}

impl std::fmt::Display for RotateReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RotateReason::SizeThreshold => write!(f, "size threshold"),
            RotateReason::Timeout => write!(f, "timeout"),
            RotateReason::Shutdown => write!(f, "graceful shutdown"),
        }
    }
}

// 主键 trait：定义 memtable 主键的行为
pub trait MemtableKey:
    Clone + Eq + std::hash::Hash + Ord + Send + Sync + std::fmt::Debug + 'static
{
    // 用于前缀索引的范围查询起点
    fn min_value() -> Self;
}

// 为常见类型实现 MemtableKey
impl MemtableKey for i64 {
    fn min_value() -> Self {
        i64::MIN
    }
}

impl MemtableKey for u64 {
    fn min_value() -> Self {
        0
    }
}

impl MemtableKey for String {
    fn min_value() -> Self {
        String::new()
    }
}

impl MemtableKey for uuid::Uuid {
    fn min_value() -> Self {
        uuid::Uuid::nil()
    }
}

// 索引匹配模式：定义索引的查询方式
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexMatch {
    Exact,  // 精确匹配
    Prefix, // 前缀匹配
}

// 索引配置：描述一个索引字段的完整信息
#[derive(Clone, Debug)]
pub struct IndexConfig {
    pub name: String,
    pub match_mode: IndexMatch,
}

impl IndexConfig {
    pub fn exact(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            match_mode: IndexMatch::Exact,
        }
    }

    pub fn prefix(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            match_mode: IndexMatch::Prefix,
        }
    }
}

// 索引值类型：使用 enum 包装常见类型
// 优点：性能好、零成本抽象、易于扩展
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IndexValue {
    I64(i64),
    U64(u64),
    I32(i32),
    U32(u32),
    String(String),
    Uuid(uuid::Uuid),
    Bool(bool),
}

impl IndexValue {
    // 提取字符串值（用于前缀索引）
    pub fn as_string(&self) -> Option<&str> {
        match self {
            IndexValue::String(s) => Some(s),
            _ => None,
        }
    }
}

// 提供便捷的构造方法
impl IndexValue {
    pub fn from<T: Into<IndexValue>>(value: T) -> Self {
        value.into()
    }
}

// 为常见类型实现 Into<IndexValue>
impl From<i64> for IndexValue {
    fn from(v: i64) -> Self {
        IndexValue::I64(v)
    }
}

impl From<u64> for IndexValue {
    fn from(v: u64) -> Self {
        IndexValue::U64(v)
    }
}

impl From<i32> for IndexValue {
    fn from(v: i32) -> Self {
        IndexValue::I32(v)
    }
}

impl From<u32> for IndexValue {
    fn from(v: u32) -> Self {
        IndexValue::U32(v)
    }
}

impl From<String> for IndexValue {
    fn from(v: String) -> Self {
        IndexValue::String(v)
    }
}

impl From<&str> for IndexValue {
    fn from(v: &str) -> Self {
        IndexValue::String(v.to_string())
    }
}

impl From<uuid::Uuid> for IndexValue {
    fn from(v: uuid::Uuid) -> Self {
        IndexValue::Uuid(v)
    }
}

impl From<bool> for IndexValue {
    fn from(v: bool) -> Self {
        IndexValue::Bool(v)
    }
}

// 前缀索引结构：使用 BTreeMap 支持范围查询
// 对于字符串前缀查询，我们存储 (string_value, primary_key) -> ()
// 这样可以通过 range() 方法快速找到所有匹配前缀的 key
#[derive(Clone, Debug)]
pub struct PrefixIndex<K: MemtableKey> {
    // BTreeMap 按字符串排序，支持范围查询
    // key: (索引值, 主键)，value: ()（仅用于去重）
    data: BTreeMap<(String, K), ()>,
}

impl<K: MemtableKey> PrefixIndex<K> {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, value: String, key: K) {
        self.data.insert((value, key), ());
    }

    pub fn remove(&mut self, value: &str, key: &K) {
        self.data.remove(&(value.to_string(), key.clone()));
    }

    // 前缀查询：返回所有匹配前缀的主键
    pub fn find_by_prefix(&self, prefix: &str) -> Vec<K> {
        let start = (prefix.to_string(), K::min_value());
        let end_prefix = Self::next_prefix(prefix);

        if let Some(end_prefix_str) = end_prefix {
            let end = (end_prefix_str, K::min_value());
            self.data
                .range(start..end)
                .map(|((_, key), _)| key.clone())
                .collect()
        } else {
            // 如果没有下一个前缀（例如 "zzz..."），则查询到末尾
            self.data
                .range(start..)
                .take_while(|((s, _), _)| s.starts_with(prefix))
                .map(|((_, key), _)| key.clone())
                .collect()
        }
    }

    // 计算下一个前缀（用于范围查询）
    // 例如：next_prefix("abc") = Some("abd")
    //      next_prefix("ab\u{10ffff}") = Some("ac")
    fn next_prefix(prefix: &str) -> Option<String> {
        let mut chars: Vec<char> = prefix.chars().collect();

        // 从后向前找到第一个可以递增的字符
        for i in (0..chars.len()).rev() {
            if let Some(next_char) = std::char::from_u32(chars[i] as u32 + 1) {
                chars[i] = next_char;
                chars.truncate(i + 1); // 移除后续字符
                return Some(chars.into_iter().collect());
            }
        }

        None // 所有字符都是最大值
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

// Memtable 结构：类似 LSM Tree 的 memtable
// 包含数据表和通用索引表
// 注意：Memtable 本身通过 RwLock 保护，所以内部使用非并发数据结构即可
// K: 主键类型, V: 值类型
pub struct Memtable<K: MemtableKey, V: MemtableValue<K>> {
    pub(crate) data: HashMap<K, Arc<V>>,
    pub(crate) indexes: HashMap<String, HashMap<IndexValue, K>>,
    pub(crate) prefix_indexes: HashMap<String, PrefixIndex<K>>, // 前缀索引
    pub(crate) tombstones: HashSet<K>,
}

impl<K: MemtableKey, V: MemtableValue<K>> Memtable<K, V> {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            indexes: HashMap::new(),
            prefix_indexes: HashMap::new(),
            tombstones: HashSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get(&self, key: &K) -> Option<Arc<V>> {
        // 如果被标记为删除，返回 None
        if self.tombstones.contains(key) {
            return None;
        }
        self.data.get(key).cloned()
    }

    // 通过索引查询
    pub fn get_by_index(&self, index_name: &str, index_value: IndexValue) -> Option<Arc<V>> {
        if let Some(index_map) = self.indexes.get(index_name) {
            if let Some(key) = index_map.get(&index_value) {
                return self.get(key);
            }
        }
        None
    }

    // 通过前缀索引查询（返回所有匹配的值）
    pub fn find_by_prefix(&self, index_name: &str, prefix: &str) -> Vec<Arc<V>> {
        if let Some(prefix_index) = self.prefix_indexes.get(index_name) {
            prefix_index
                .find_by_prefix(prefix)
                .into_iter()
                .filter_map(|key| self.get(&key))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn insert(&mut self, key: K, value: Arc<V>) {
        // 如果已存在，先清理旧索引（需要先克隆以避免借用冲突）
        let old_value_opt = self.data.get(&key).cloned();
        if let Some(old_value) = old_value_opt {
            self.remove_indexes(&*old_value);
            self.remove_prefix_indexes(&key, &*old_value);
        }

        // 插入数据（清除 tombstone 标记）
        self.tombstones.remove(&key);
        self.data.insert(key.clone(), value.clone());

        // 从 MemtableValue 中提取索引并建立索引（统一处理）
        self.build_indexes(key.clone(), &*value);
        // build_prefix_indexes 已合并到 build_indexes 中
    }

    pub fn add_tombstone(&mut self, key: K) {
        self.tombstones.insert(key);
    }

    // 从 MemtableValue 中提取索引并建立索引
    fn build_indexes(&mut self, key: K, value: &V) {
        for (index_name, index_value, match_mode) in value.get_indexes() {
            match match_mode {
                IndexMatch::Exact => {
                    // 精确匹配索引
                    self.indexes
                        .entry(index_name.to_string())
                        .or_insert_with(HashMap::new)
                        .insert(index_value, key.clone());
                }
                IndexMatch::Prefix => {
                    // 前缀匹配索引（仅支持字符串）
                    if let Some(string_value) = index_value.as_string() {
                        self.prefix_indexes
                            .entry(index_name.to_string())
                            .or_insert_with(PrefixIndex::new)
                            .insert(string_value.to_string(), key.clone());
                    }
                }
            }
        }
    }

    // 移除索引
    fn remove_indexes(&mut self, value: &V) {
        // 收集所有需要移除的索引信息
        for (index_name, index_value, match_mode) in value.get_indexes() {
            match match_mode {
                IndexMatch::Exact => {
                    if let Some(index_map) = self.indexes.get_mut(index_name) {
                        index_map.remove(&index_value);
                    }
                }
                IndexMatch::Prefix => {
                    // 前缀索引的移除需要 key，在 remove_prefix_indexes 中处理
                }
            }
        }

        // 清理空的索引表
        self.indexes
            .retain(|_name, index_map| !index_map.is_empty());
    }

    // 移除前缀索引
    fn remove_prefix_indexes(&mut self, key: &K, value: &V) {
        for (index_name, index_value, match_mode) in value.get_indexes() {
            if match_mode == IndexMatch::Prefix {
                if let Some(string_value) = index_value.as_string() {
                    if let Some(prefix_index) = self.prefix_indexes.get_mut(index_name) {
                        prefix_index.remove(string_value, key);
                    }
                }
            }
        }

        // 清理空的前缀索引表
        self.prefix_indexes
            .retain(|_name, prefix_index| !prefix_index.is_empty());
    }

    pub fn remove(&mut self, key: &K) -> Option<Arc<V>> {
        // 先获取并克隆旧值，以便移除索引
        let old_value_opt = self.data.get(key).cloned();
        if let Some(ref old_value) = old_value_opt {
            // 移除旧索引
            self.remove_indexes(&**old_value);
            self.remove_prefix_indexes(key, &**old_value);
        }

        // 添加 tombstone 标记
        self.tombstones.insert(key.clone());

        // 从数据表中移除并返回
        self.data.remove(key)
    }

    // 检查是否包含 key（用于判断是否需要增加计数器）
    // 注意：如果项存在但是 tombstone，仍然返回 true（因为需要更新计数器）
    pub fn contains_key(&self, key: &K) -> bool {
        self.data.contains_key(key)
    }

    // 收集所有数据用于 flush（只收集非 tombstone 的项）
    pub fn collect_items(&self) -> Vec<(K, Arc<V>)> {
        self.data
            .iter()
            .filter_map(|(key, item)| {
                if self.tombstones.contains(key) {
                    None
                } else {
                    Some((key.clone(), item.clone()))
                }
            })
            .collect()
    }

    // 收集所有 tombstones
    pub fn collect_tombstones(&self) -> Vec<K> {
        self.data
            .iter()
            .filter_map(|(key, _)| {
                if self.tombstones.contains(key) {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

// 为 Memtable 实现 Clone（因为内部都是可 Clone 的类型）
impl<K: MemtableKey, V: MemtableValue<K>> Clone for Memtable<K, V> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            indexes: self.indexes.clone(),
            prefix_indexes: self.prefix_indexes.clone(),
            tombstones: self.tombstones.clone(),
        }
    }
}

// MemtableValue trait：定义 memtable 值的行为
pub trait MemtableValue<K: MemtableKey>: Send + Sync + Clone + 'static {
    // 获取主键
    fn get_key(&self) -> K;

    // 统一的索引接口：返回 (索引名, 索引值, 匹配模式)
    // 这个方法替代了之前的 get_indexes() 和 get_prefix_indexes()
    fn get_indexes(&self) -> Vec<(&str, IndexValue, IndexMatch)>;

    // 获取单个索引值（用于特定场景）
    fn get_index(&self, index_name: &str) -> IndexValue;
}

// MemtablePersister trait：定义持久化行为
// 负责将 memtable 中的数据持久化到存储层（数据库/文件等）
#[async_trait]
pub trait MemtablePersister<K: MemtableKey, V: MemtableValue<K>>:
    Send + Sync + Clone + 'static
{
    /// 持久化单个数据项
    async fn persist(&self, key: K, value: Arc<V>) -> Result<(), String>;

    /// 删除单个数据项
    async fn remove(&self, key: K) -> Result<(), String>;

    /// 批量持久化数据项（默认实现：逐个调用 persist）
    async fn persist_batch(&self, items: Vec<(K, Arc<V>)>) -> Result<(), String> {
        for (key, value) in items {
            self.persist(key, value).await?;
        }
        Ok(())
    }

    /// 批量删除数据项（默认实现：逐个调用 remove）
    async fn remove_batch(&self, keys: Vec<K>) -> Result<(), String> {
        for key in keys {
            self.remove(key).await?;
        }
        Ok(())
    }
}

// MemtableContext：管理 memtable 的上下文信息
// 职责：
// 1. 管理 active memtable 的生命周期
// 2. 控制大小阈值触发的旋转
// 3. 控制超时触发的旋转（自动 flush）
// 4. 协调数据持久化
// 5. 通过 oneshot channel 实现背压控制，防止 OOM
pub struct MemtableContext<K: MemtableKey, V: MemtableValue<K>, P: MemtablePersister<K, V>> {
    name: String,
    active_memtable: Arc<RwLock<Memtable<K, V>>>,
    immutable_memtable: Arc<RwLock<Option<Arc<RwLock<Memtable<K, V>>>>>>,
    active_size: Arc<AtomicUsize>,
    active_threshold_size: usize,
    persister: Arc<P>,
    rotate_time: Arc<RwLock<Option<Instant>>>,
    flush_timeout: Duration,
}

impl<K: MemtableKey, V: MemtableValue<K>, P: MemtablePersister<K, V>> MemtableContext<K, V, P> {
    pub fn new(
        name: String,
        memtable: Arc<RwLock<Memtable<K, V>>>,
        size: Arc<AtomicUsize>,
        threshold_size: usize,
        persister: Arc<P>,
        flush_timeout: Duration,
    ) -> Self {
        Self {
            name,
            active_memtable: memtable,
            immutable_memtable: Arc::new(RwLock::new(None)),
            active_size: size,
            active_threshold_size: threshold_size,
            persister,
            rotate_time: Arc::new(RwLock::new(Some(Instant::now()))),
            flush_timeout,
        }
    }

    // 获取 persister 引用
    pub fn persister(&self) -> &Arc<P> {
        &self.persister
    }

    // 直接 flush immutable memtable
    pub async fn flush_immutable(&self, memtable: &Memtable<K, V>) -> Result<(), String> {
        let items = memtable.collect_items();
        let tombstones = memtable.collect_tombstones();

        // 使用 persister 持久化数据
        self.persister.persist_batch(items).await?;
        self.persister.remove_batch(tombstones).await?;

        Ok(())
    }

    // 检查是否需要旋转（达到阈值）
    pub fn should_rotate(&self) -> bool {
        self.active_size.load(Ordering::Relaxed) >= self.active_threshold_size
    }

    // 触发旋转（通过设置 rotate_time）
    pub async fn trigger_rotate(&self) {
        let mut rotate_time = self.rotate_time.write().await;
        *rotate_time = Some(Instant::now());
    }

    // 重置旋转时间（通常在旋转完成后调用）
    pub async fn reset_rotate_time(&self) {
        let mut rotate_time = self.rotate_time.write().await;
        *rotate_time = None;
    }

    // 获取旋转时间
    pub async fn get_rotate_time(&self) -> Option<Instant> {
        let rotate_time = self.rotate_time.read().await;
        *rotate_time
    }

    // 更新 active memtable 的创建时间（通常在创建新的 active memtable 时调用）
    pub async fn update_active_memtable_created_at(&self) {
        let mut rotate_time = self.rotate_time.write().await;
        *rotate_time = Some(Instant::now());
    }

    // 获取当前大小
    pub fn size(&self) -> usize {
        self.active_size.load(Ordering::Relaxed)
    }

    // 获取 memtable 引用
    pub fn memtable(&self) -> &Arc<RwLock<Memtable<K, V>>> {
        &self.active_memtable
    }

    // 获取 size 引用
    pub fn size_ref(&self) -> &Arc<AtomicUsize> {
        &self.active_size
    }

    // 克隆 context（用于异步任务）
    fn clone_without_generic(&self) -> Self {
        Self {
            name: self.name.clone(),
            active_memtable: self.active_memtable.clone(),
            immutable_memtable: self.immutable_memtable.clone(),
            active_size: self.active_size.clone(),
            active_threshold_size: self.active_threshold_size,
            persister: self.persister.clone(),
            rotate_time: self.rotate_time.clone(),
            flush_timeout: self.flush_timeout,
        }
    }

    // 获取 flush 超时时间
    pub fn flush_timeout(&self) -> Duration {
        self.flush_timeout
    }

    // 获取阈值大小
    pub fn threshold_size(&self) -> usize {
        self.active_threshold_size
    }

    // 检查 active memtable 是否超时
    pub async fn should_flush_by_timeout(&self, flush_timeout: Duration) -> bool {
        let rotate_time = self.rotate_time.read().await;
        if let Some(created) = *rotate_time {
            let elapsed = created.elapsed();
            if elapsed >= flush_timeout {
                let size = self.active_size.load(Ordering::Relaxed);
                return size > 0;
            }
        }
        false
    }

    // 插入数据（高层封装，自动处理计数和旋转）
    pub async fn insert(&self, key: K, value: Arc<V>) -> Result<(), MemtableError> {
        // 1. 插入数据
        let mut memtable = self.active_memtable.write().await;
        let was_new = !memtable.contains_key(&key);
        memtable.insert(key, value);
        drop(memtable); // 显式释放锁

        // 2. 更新计数器
        if was_new {
            let new_size = self.active_size.fetch_add(1, Ordering::Relaxed) + 1;

            // 3. 检查是否需要旋转
            if new_size >= self.active_threshold_size {
                let active_state = ActiveState::new(Arc::new(self.clone_without_generic()));
                if let Some((_, old_size)) = active_state.rotate(RotateReason::SizeThreshold).await
                {
                    info!(
                        "[{}] Memtable auto-rotated: {} items flushed to database",
                        self.name, old_size
                    );
                }
            }
        }

        Ok(())
    }

    // 原子性更新或插入（解决 read-modify-write 竞态条件）
    // 使用单个写锁保护整个操作，确保线程安全
    pub async fn update_or_insert<F>(&self, key: K, updater: F) -> Result<(), MemtableError>
    where
        F: FnOnce(Option<Arc<V>>) -> Arc<V>,
    {
        // 1. 获取写锁，保护整个 read-modify-write 操作
        let mut memtable = self.active_memtable.write().await;
        let current = memtable.get(&key);
        let was_new = current.is_none();

        // 2. 调用 updater 函数生成新值
        let new_value = updater(current);

        // 3. 插入新值
        memtable.insert(key, new_value);
        drop(memtable); // 显式释放锁

        // 4. 更新计数器并检查旋转
        if was_new {
            let new_size = self.active_size.fetch_add(1, Ordering::Relaxed) + 1;

            if new_size >= self.active_threshold_size {
                let active_state = ActiveState::new(Arc::new(self.clone_without_generic()));
                if let Some((_, old_size)) = active_state.rotate(RotateReason::SizeThreshold).await
                {
                    info!(
                        "[{}] Memtable auto-rotated: {} items flushed to database",
                        self.name, old_size
                    );
                }
            }
        }

        Ok(())
    }

    // 查询数据（通过主键）
    pub async fn get(&self, key: &K) -> Option<Arc<V>> {
        // 先从 active memtable 查询
        let memtable = self.active_memtable.read().await;
        if let Some(value) = memtable.get(key) {
            return Some(value);
        }
        drop(memtable);

        // 如果 active memtable 中没有找到，再从 immutable memtable 查询
        let immutable = self.immutable_memtable.read().await;
        if let Some(ref immutable_memtable) = *immutable {
            let memtable = immutable_memtable.read().await;
            return memtable.get(key);
        }
        None
    }

    // 查询数据（通过索引）
    pub async fn get_by_index(&self, index_name: &str, index_value: IndexValue) -> Option<Arc<V>> {
        // 先从 active memtable 查询
        let memtable = self.active_memtable.read().await;
        if let Some(value) = memtable.get_by_index(index_name, index_value.clone()) {
            return Some(value);
        }
        drop(memtable);

        // 如果 active memtable 中没有找到，再从 immutable memtable 查询
        let immutable = self.immutable_memtable.read().await;
        if let Some(ref immutable_memtable) = *immutable {
            let memtable = immutable_memtable.read().await;
            return memtable.get_by_index(index_name, index_value);
        }
        None
    }

    // 前缀查询（通过前缀索引）
    pub async fn find_by_prefix(&self, index_name: &str, prefix: &str) -> Vec<Arc<V>> {
        let mut results = Vec::new();

        // 从 active memtable 查询
        let memtable = self.active_memtable.read().await;
        results.extend(memtable.find_by_prefix(index_name, prefix));
        drop(memtable);

        // 从 immutable memtable 查询
        let immutable = self.immutable_memtable.read().await;
        if let Some(ref immutable_memtable) = *immutable {
            let memtable = immutable_memtable.read().await;
            results.extend(memtable.find_by_prefix(index_name, prefix));
        }

        results
    }

    // 删除数据（从 active memtable 中移除）
    pub async fn delete(&self, key: &K) -> Result<(), MemtableError> {
        let mut memtable = self.active_memtable.write().await;
        if memtable.remove(key).is_some() {
            self.active_size.fetch_sub(1, Ordering::Relaxed);
        }
        Ok(())
    }

    // 启动自动 flush 定时器
    // 这个方法应该在 MemtableContext 创建后立即调用
    // 它会在后台启动一个任务，定期检查是否需要基于超时触发 flush
    pub fn start_auto_flush_timer(self: &Arc<Self>) {
        let context = Arc::clone(self);
        let flush_timeout = self.flush_timeout;
        let name = self.name.clone();

        info!(
            "[{}] Auto-flush timer starting with interval: {:?}",
            name,
            flush_timeout / 2
        );

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(flush_timeout / 2);
            //let mut tick_count = 0u64;
            loop {
                interval.tick().await;
                /*
                tick_count += 1;

                // 每10次 tick 打印一次心跳日志（用于调试）
                if tick_count % 10 == 0 {
                    let size = context.size();
                    info!(
                        "[{}] Memtable heartbeat: {} items in active memtable",
                        name, size
                    );
                }
                */

                // 检查是否超时
                if context.should_flush_by_timeout(flush_timeout).await {
                    info!(
                        "[{}] Memtable flush timeout reached, triggering rotation...",
                        name
                    );

                    // 触发旋转
                    let active_state = ActiveState::new(Arc::clone(&context));
                    active_state.rotate(RotateReason::Timeout).await;
                }
            }
        });
    }

    /// 优雅关闭：强制 flush active memtable 中的所有数据
    ///
    /// 应该在应用关闭时调用，确保所有数据都持久化到数据库
    ///
    /// 返回 flush 的数据条数，如果 memtable 为空则返回 None
    ///
    /// # Example
    /// ```rust
    /// // 在应用关闭时调用
    /// if let Some(count) = memtable_context.shutdown_gracefully().await {
    ///     println!("Flushed {} items during shutdown", count);
    /// }
    /// ```
    pub async fn shutdown_gracefully(&self) -> Option<usize> {
        let size = self.active_size.load(Ordering::Relaxed);

        if size == 0 {
            info!(
                "[{}] Memtable is empty, no need to flush during shutdown",
                self.name
            );
            return None;
        }

        info!(
            "[{}] Graceful shutdown: attempting to flush {} items from active memtable",
            self.name, size
        );

        // 强制旋转 active memtable（即使没有达到阈值）
        let active_state = ActiveState::new(Arc::new(self.clone_without_generic()));
        if let Some((_, flushed_size)) = active_state.rotate(RotateReason::Shutdown).await {
            info!(
                "[{}] Graceful shutdown: successfully flushed {} items",
                self.name, flushed_size
            );
            Some(flushed_size)
        } else {
            warn!(
                "[{}] Graceful shutdown: no items to flush (memtable was empty)",
                self.name
            );
            None
        }
    }
}

// Active 状态：当前可写的 memtable
pub struct ActiveState<K: MemtableKey, V: MemtableValue<K>, P: MemtablePersister<K, V>> {
    context: Arc<MemtableContext<K, V, P>>,
}

impl<K: MemtableKey, V: MemtableValue<K>, P: MemtablePersister<K, V>> ActiveState<K, V, P> {
    pub fn new(context: Arc<MemtableContext<K, V, P>>) -> Self {
        Self { context }
    }

    // 旋转 memtable：将当前的 active memtable 变为 immutable，创建新的 active
    // 返回旧的 memtable 和大小
    //
    // 关键改进：使用 oneshot channel 确保每次 rotate 的 flush 完成后才能继续
    // 这防止了 immutable memtables 堆积导致的 OOM
    pub async fn rotate(&self, reason: RotateReason) -> Option<(Memtable<K, V>, usize)> {
        let start_time = Instant::now();
        let mut memtable = self.context.memtable().write().await;
        let old_size = memtable.len();
        let name = &self.context.name;

        if old_size == 0 {
            // 如果 memtable 为空，不需要旋转
            info!(
                "[{}] Memtable rotation skipped (reason: {}, size: 0)",
                name, reason
            );
            return None;
        }

        info!(
            "[{}] Memtable rotation started (reason: {}, size: {} items)",
            name, reason, old_size
        );

        // 创建新的 memtable
        let new_memtable = Memtable::<K, V>::new();
        // 替换 memtable（move，不 clone）
        let old_memtable = std::mem::replace(&mut *memtable, new_memtable);

        // ⚠️ 关键：先克隆 old_memtable，稍后再设置 immutable_memtable
        let old_memtable_for_immutable = old_memtable.clone();

        // 重置大小计数器
        self.context.size_ref().store(0, Ordering::Relaxed);

        // ✅ 显式释放 active_memtable 写锁，允许新数据写入新的 active memtable
        drop(memtable);

        // ✅ 关键改进：为这次 rotate 创建专属的 oneshot channel
        // 避免多个 rotate 共享 receiver 导致的信号混乱和死锁
        let (tx, rx) = oneshot::channel();

        // ✅ 在释放 active_memtable 锁之后，再获取其他锁，避免死锁
        // 将旧的 memtable 存储为 immutable memtable，供查询使用
        {
            let mut immutable = self.context.immutable_memtable.write().await;
            *immutable = Some(Arc::new(RwLock::new(old_memtable_for_immutable)));
        }

        // 重置旋转时间，开始新的超时周期
        // ⚠️ 关键修复：必须在等待 flush 之前释放此锁，否则会导致死锁
        // 因为 auto-flush timer 会调用 should_flush_by_timeout() 需要获取 rotate_time 读锁
        {
            let mut rotate_time = self.context.rotate_time.write().await;
            *rotate_time = Some(Instant::now());
        } // 锁在这里立即释放

        // 在后台异步 flush immutable memtable
        let context = self.context.clone_without_generic();
        let memtable_to_flush = old_memtable.clone();
        let name_clone = name.clone();
        tokio::spawn(async move {
            if let Err(e) = context.flush_immutable(&memtable_to_flush).await {
                error!("[{}] Failed to flush memtable: {}", name_clone, e);
            }

            // flush 完成后清除 immutable memtable
            {
                let mut immutable = context.immutable_memtable.write().await;
                *immutable = None;
            }

            // flush 完成后发送信号给这次 rotate 专属的 receiver
            // 如果 receiver 已经 dropped（不太可能），这里会失败但不影响正确性
            let _ = tx.send(());
        });

        // 等待 flush 完成信号（背压控制点）
        // ⚠️ 关键：此时所有锁都已释放，不会阻塞其他操作
        // 优点：
        // 1. 每次 rotate 有独立的 channel，不会信号混乱
        // 2. 如果 flush 很快，这里几乎不阻塞
        // 3. 如果 flush 很慢，这里会阻塞，防止连续 rotate 导致 OOM
        // 4. 阻塞期间，新数据仍然可以写入新的 active memtable（写锁已释放）
        // 5. 其他需要读取 rotate_time 的操作不会被阻塞（如 auto-flush timer）
        match rx.await {
            Ok(()) => {
                info!(
                    "[{}] Memtable rotation completed (reason: {}, size: {} items, elapsed: {:?})",
                    name,
                    reason,
                    old_size,
                    start_time.elapsed()
                );
            }
            Err(_) => {
                // Sender dropped，说明 flush 任务异常退出
                warn!(
                    "[{}] Memtable rotation failed - flush task terminated unexpectedly (reason: {}, size: {} items)",
                    name, reason, old_size
                );
            }
        }

        Some((old_memtable, old_size))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MemtableError {
    #[error("State error: {0}")]
    StateError(String),
}

// MemtableState trait：定义 memtable 状态的行为
#[async_trait]
pub trait MemtableState<K: MemtableKey, V: MemtableValue<K>, P: MemtablePersister<K, V>>:
    Send + Sync
{
    // 插入数据
    async fn insert(
        &mut self,
        context: &MemtableContext<K, V, P>,
        key: K,
        value: Arc<V>,
    ) -> Result<(), MemtableError>;

    // 查询数据（通过 key）
    async fn get_by_key(&self, context: &MemtableContext<K, V, P>, key: &K) -> Option<Arc<V>>;

    // 通过索引查询
    async fn get_by_index(
        &self,
        context: &MemtableContext<K, V, P>,
        index_name: &str,
        index_value: IndexValue,
    ) -> Option<Arc<V>>;

    // 删除数据
    async fn delete(
        &mut self,
        context: &MemtableContext<K, V, P>,
        key: &K,
    ) -> Result<(), MemtableError>;

    // 检查是否包含 key
    async fn contains_key(&self, context: &MemtableContext<K, V, P>, key: &K) -> bool;
}

#[async_trait]
impl<K: MemtableKey, V: MemtableValue<K>, P: MemtablePersister<K, V>> MemtableState<K, V, P>
    for ActiveState<K, V, P>
{
    async fn insert(
        &mut self,
        context: &MemtableContext<K, V, P>,
        key: K,
        value: Arc<V>,
    ) -> Result<(), MemtableError> {
        let mut memtable = context.memtable().write().await;
        let was_new = !memtable.contains_key(&key);
        memtable.insert(key, value);
        if was_new {
            context.size_ref().fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    async fn get_by_key(&self, context: &MemtableContext<K, V, P>, key: &K) -> Option<Arc<V>> {
        let memtable = context.memtable().read().await;
        memtable.get(key)
    }

    async fn get_by_index(
        &self,
        context: &MemtableContext<K, V, P>,
        index_name: &str,
        index_value: IndexValue,
    ) -> Option<Arc<V>> {
        let memtable = context.memtable().read().await;
        memtable.get_by_index(index_name, index_value)
    }

    async fn delete(
        &mut self,
        context: &MemtableContext<K, V, P>,
        key: &K,
    ) -> Result<(), MemtableError> {
        let mut memtable = context.memtable().write().await;
        memtable.remove(key);
        context.size_ref().fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn contains_key(&self, context: &MemtableContext<K, V, P>, key: &K) -> bool {
        let memtable = context.memtable().read().await;
        memtable.contains_key(key)
    }
}

// Immutable 状态：等待 flush 的 memtable（只读，但可以添加 tombstone）
pub struct ImmutableState<K: MemtableKey, V: MemtableValue<K>, P: MemtablePersister<K, V>> {
    _phantom: std::marker::PhantomData<(K, V, P)>,
}

#[async_trait]
impl<K: MemtableKey, V: MemtableValue<K>, P: MemtablePersister<K, V>> MemtableState<K, V, P>
    for ImmutableState<K, V, P>
{
    async fn insert(
        &mut self,
        _context: &MemtableContext<K, V, P>,
        _key: K,
        _value: Arc<V>,
    ) -> Result<(), MemtableError> {
        Err(MemtableError::StateError(
            "Immutable state does not support insert".to_string(),
        ))
    }
    async fn get_by_key(&self, context: &MemtableContext<K, V, P>, key: &K) -> Option<Arc<V>> {
        let memtable = context.memtable().read().await;
        memtable.get(key)
    }

    async fn get_by_index(
        &self,
        context: &MemtableContext<K, V, P>,
        index_name: &str,
        index_value: IndexValue,
    ) -> Option<Arc<V>> {
        let memtable = context.memtable().read().await;
        if let Some(index_map) = memtable.indexes.get(index_name) {
            if let Some(key) = index_map.get(&index_value) {
                memtable.get(key)
            } else {
                None
            }
        } else {
            None
        }
    }
    async fn delete(
        &mut self,
        _context: &MemtableContext<K, V, P>,
        _key: &K,
    ) -> Result<(), MemtableError> {
        Err(MemtableError::StateError(
            "Immutable state does not support delete".to_string(),
        ))
    }
    async fn contains_key(&self, context: &MemtableContext<K, V, P>, key: &K) -> bool {
        let memtable = context.memtable().read().await;
        memtable.contains_key(key)
    }
}
