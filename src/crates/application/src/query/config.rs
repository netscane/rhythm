pub trait AppQueryConfig {
    fn indexgroups(&self) -> String;
}

/// 封面艺术配置
pub trait CoverArtConfig {
    /// 获取封面文件名通配符列表（按优先级排序，越靠前优先级越高）
    fn cover_art_wildcards(&self) -> Vec<String>;
    
    /// 获取艺术家占位图路径
    fn artist_placeholder_path(&self) -> Option<String>;
    
    /// 获取专辑占位图路径
    fn album_placeholder_path(&self) -> Option<String>;
}
