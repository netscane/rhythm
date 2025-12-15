use actix_files::Files;
use actix_web::web;

/// 配置静态资源路由，映射 /resources 到静态文件目录
pub fn configure_service(cfg: &mut web::ServiceConfig) {
    cfg.service(Files::new("/resources", "resources").show_files_listing());
}
