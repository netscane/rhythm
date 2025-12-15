//pub mod user;
use super::consts;
use super::AppState;
use crate::middleware::other;
use actix_web::{
    http::StatusCode, middleware::from_fn, web, web::Json, web::Path, web::Query, HttpResponse,
    Responder, Scope,
};
use infra::repository::postgres::{
    command::db_data::{
        album::ActiveModel as AlbumActiveModel, artist::ActiveModel as ArtistActiveModel,
        genre::ActiveModel as GenreActiveModel, player::ActiveModel as PlayerActiveModel,
        user::ActiveModel as UserActiveModel,
    },
    restful::{RestfulError as RepositoryError, RestfulRepository},
};
use log::info;
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, EntityTrait, IntoActiveModel};
use serde::Serialize;
use serde_json::Value;
use std::marker::PhantomData;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RestfulError {
    #[error("{0}")]
    ResourceNotFound(String),
    #[error(transparent)]
    OperateDatabase(#[from] sea_orm::DbErr),
    #[error("{0}")]
    Unknown(String),
}

impl From<RepositoryError> for RestfulError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::PrimaryKeyNotFound(msg) => RestfulError::ResourceNotFound(msg),
            RepositoryError::DbErr(msg) => RestfulError::OperateDatabase(msg),
        }
    }
}

impl actix_web::error::ResponseError for RestfulError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ResourceNotFound(_) => StatusCode::NOT_FOUND,
            Self::OperateDatabase(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let message = self.to_string();
        HttpResponse::build(self.status_code()).body(message)
    }
}

pub struct Restful<T> {
    _marker: PhantomData<T>,
}

impl<T> Restful<T>
where
    T: ActiveModelTrait + ActiveModelBehavior + Send + 'static + Sync,
    <T::Entity as EntityTrait>::Model: IntoActiveModel<T> + Serialize + Sync,
    for<'de> <T::Entity as EntityTrait>::Model: serde::de::Deserialize<'de>,
{
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
    async fn create(
        state: web::Data<AppState>,
        Json(data): Json<<T::Entity as EntityTrait>::Model>,
    ) -> Result<HttpResponse, RestfulError> {
        let repo: RestfulRepository<T> = RestfulRepository::new(state.db.clone());
        repo.create(data).await?;
        Ok(HttpResponse::Created().finish())
    }

    async fn update(
        state: web::Data<AppState>,
        path: Path<i64>,
        Json(data): Json<<T::Entity as EntityTrait>::Model>,
    ) -> Result<HttpResponse, RestfulError> {
        let repo: RestfulRepository<T> = RestfulRepository::new(state.db.clone());
        let pk = path.into_inner();
        repo.update(pk, data).await?;
        Ok(HttpResponse::Ok().into())
    }

    async fn list(
        state: web::Data<AppState>,
        Query(query): Query<Value>,
    ) -> Result<Json<Value>, RestfulError> {
        let page_size = Self::get_page_size(&query);
        let page_num = Self::get_page_num(&query);
        let repo: RestfulRepository<T> = RestfulRepository::new(state.db.clone());
        let items = repo.list(page_size, page_num).await?;
        Ok(Json(serde_json::json!(items)))
    }

    async fn retrieve(
        state: web::Data<AppState>,
        path: web::Path<i64>,
    ) -> Result<Json<Value>, RestfulError> {
        let pk = path.into_inner();
        let repo: RestfulRepository<T> = RestfulRepository::new(state.db.clone());

        let instance = repo.get_by_id(pk).await?;
        Ok(Json(serde_json::json!(instance)))
    }

    async fn delete(
        state: web::Data<AppState>,
        path: web::Path<i64>,
    ) -> Result<HttpResponse, RestfulError> {
        let pk = path.into_inner();
        let repo: RestfulRepository<T> = RestfulRepository::new(state.db.clone());
        repo.delete_by_id(pk).await?;
        Ok(HttpResponse::Ok().into())
    }

    fn rx(&self, nest_prefix: &str) -> Scope
    where
        Self: Send + 'static,
    {
        info!("http config for {}", nest_prefix);
        web::scope(nest_prefix)
            .service(
                web::resource("/{id}")
                    .route(web::get().to(Self::retrieve))
                    .route(web::put().to(Self::update))
                    .route(web::delete().to(Self::delete)),
            )
            .service(
                web::resource("")
                    .route(web::get().to(Self::list)) // 修正这里
                    .route(web::post().to(Self::create)),
            )
    }

    fn get_page_size(query: &Value) -> i64 {
        // 实现获取页面大小的逻辑
        query
            .get("page_size")
            .and_then(|v| v.as_i64())
            .unwrap_or(10)
    }

    fn get_page_num(query: &Value) -> i64 {
        // 实现获取页面编号的逻辑
        query.get("page_num").and_then(|v| v.as_i64()).unwrap_or(0)
    }
}

async fn keepalive(_path: web::Path<String>) -> impl Responder {
    // 这里可以使用 path 变量来处理捕获的路径
    static KEEPALIVE_RESPONSE: &str = r#"{{"response":"ok", "id":"keepalive"}}"#;
    HttpResponse::Ok()
        .content_type("application/json")
        .body(KEEPALIVE_RESPONSE)
}
pub fn scope_keepalive() -> Scope {
    web::scope("/keepalive").route("/{tail:.*}", web::get().to(keepalive))
}
pub fn configure_service(svc: &mut web::ServiceConfig) {
    let rx_user: Restful<UserActiveModel> = Restful::new();
    let rx_album: Restful<AlbumActiveModel> = Restful::new();
    let rx_artist: Restful<ArtistActiveModel> = Restful::new();
    let rx_player: Restful<PlayerActiveModel> = Restful::new();
    let rx_genre: Restful<GenreActiveModel> = Restful::new();
    svc.service(
        web::scope(consts::URL_PATH_NATIVE_API)
            /*
            .wrap(from_fn(move |req, next| {
                other::update_last_access(req, next)
            }))*/
            .wrap(from_fn(move |req, next| other::authenticator(req, next)))
            .service(rx_user.rx("/user"))
            .service(rx_album.rx("/album"))
            .service(rx_artist.rx("/artist"))
            .service(rx_player.rx("/player"))
            .service(rx_genre.rx("/genre"))
            .service(scope_keepalive()),
    );
}
