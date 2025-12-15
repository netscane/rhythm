use actix_web::middleware::Logger;
use actix_web::{middleware::from_fn, web, App, HttpServer};

use infra::config::AppConfigImpl;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

use server::middleware::{jwt_verify, other};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 配置日志同时输出到控制台和文件
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    // 创建文件 appender
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S%.3f)} [{l}] {m}{n}",
        )))
        .build("app.log")
        .unwrap();

    // 配置 log4rs：同时输出到控制台和文件
    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .appender(Appender::builder().build(
            "stdout",
            Box::new(log4rs::append::console::ConsoleAppender::builder().build()),
        ))
        .build(
            Root::builder()
                .appender("file")
                .appender("stdout")
                .build(log_level.parse().unwrap_or(log::LevelFilter::Info)),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();
    let cfg = AppConfigImpl::load().unwrap();
    let server_cfg = cfg.server();
    let db = server::AppState::init_db(&cfg.database_url()).await;

    let mut app_state = server::AppState::new(db.clone(), cfg).await;
    server::init_admin_user(&app_state).await;
    server::setup_event_bus(&mut app_state).await;
    let app_state = web::Data::new(app_state);
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}"))
            // auth API 不需要 JWT 验证
            .service(server::auth::configure_service())
            // Subsonic API 使用自己的认证方式，不需要 JWT
            .configure(server::subsonic::configure_service)
            // 需要 JWT 验证的路由
            .service(
                web::scope("")
                    .configure(server::native_api::configure_service)
                    .configure(server::resources::configure_service)
                    .wrap(jwt_verify::JwtVerifier {})
                    .wrap(from_fn(other::auth_header_mapper))
                    .wrap(from_fn(other::client_unique_id)),
            )
            .wrap(other::cors())
    })
    .bind((server_cfg.host.as_str(), server_cfg.port))?
    .run()
    .await
}
