use actix_files::Files;
use actix_web::{web, HttpResponse};
use infra::config::ServerConfig;
use std::path::Path;

/// 配置静态资源路由，映射 /resources 到静态文件目录
pub fn configure_service(cfg: &mut web::ServiceConfig) {
    cfg.service(Files::new("/resources", "resources").show_files_listing());
}

/// 配置 UI 静态文件服务
pub fn configure_ui_service(cfg: &mut web::ServiceConfig, server_config: &ServerConfig) {
    let ui_path = server_config.ui_path.clone();
    let ui_base_path = server_config.ui_base_path.clone();
    let index_file = Path::new(&ui_path).join("index.html");

    // 检查 UI 目录是否存在
    if !Path::new(&ui_path).exists() {
        log::warn!(
            "UI directory '{}' not found, UI service disabled",
            ui_path
        );
        return;
    }

    // 根路径重定向到 UI
    let redirect_path = ui_base_path.clone();
    cfg.route(
        "/",
        web::get().to(move || {
            let path = redirect_path.clone();
            async move { HttpResponse::Found().insert_header(("Location", path)).finish() }
        }),
    );

    // UI 静态文件服务
    if let Ok(named_file) = actix_files::NamedFile::open(&index_file) {
        cfg.service(
            Files::new(&ui_base_path, &ui_path)
                .index_file("index.html")
                // SPA fallback: 对于所有未匹配的路由，返回 index.html
                .default_handler(named_file),
        );
    } else {
        log::warn!(
            "UI index file '{}' not found, UI service disabled",
            index_file.display()
        );
    }
}
