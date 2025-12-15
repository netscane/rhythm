use crate::{consts, AppState};
use actix_service::{forward_ready, Service, Transform};
use actix_web::{
    body::MessageBody,
    dev::ServiceRequest,
    dev::ServiceResponse,
    http::header::{HeaderName, HeaderValue},
    middleware::Next,
    web, Error, HttpMessage, HttpRequest,
};
use application::auth::TokenService;
use application::auth::UserClaims;
use application::error::AppError;
use futures::future::{ok, LocalBoxFuture, Ready};
use infra::auth::AuthConfig;
use infra::auth::JwtTokenService;
use std::rc::Rc;
use std::sync::Arc;
use url::Url;

use thiserror::Error;
#[derive(Error, Debug)]
pub enum JwtError {
    #[error("token is unauthorized")]
    Unauthorized(#[from] AppError),
    #[error("token is expired")]
    Expired,
    #[error("token nbf validation failed")]
    NBFInvalid,
    #[error("token iat validation failed")]
    IATInvalid,
    #[error("invalid key")]
    InvalidKey,
    #[error("no token found")]
    NoTokenFound,
    #[error("algorithm mismatch")]
    AlgoInvalid,
    #[error("{0}")]
    OtherError(String),
}

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct JwtVerifier {}

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for JwtVerifier
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtVerifyMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(JwtVerifyMiddleware {
            service: Rc::new(service),
        })
    }
}

pub struct JwtVerifyMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for JwtVerifyMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);
    fn call(&self, req: ServiceRequest) -> Self::Future {
        println!("Hi from start. You requested: {}", req.path());
        let state = Arc::clone(req.app_data::<web::Data<AppState>>().unwrap());
        let service = self.service.clone();
        let (http_request, payload) = req.into_parts();
        let fut = async move {
            let token_finders: Vec<TokenFinder> = vec![token_from_query, token_from_header];
            match verify_jwt(&state, &http_request, &token_finders) {
                Ok(claims) => {
                    let req = ServiceRequest::from_parts(http_request, payload);
                    req.extensions_mut().insert(claims);
                    service.call(req).await
                }
                Err(_) => Err(actix_web::error::ErrorUnauthorized("Unauthorized")),
            }
        };
        Box::pin(fut)
    }
}

// 提取令牌的函数类型
type TokenFinder = fn(req: &HttpRequest) -> Option<String>;

fn token_from_header(req: &HttpRequest) -> Option<String> {
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            // 假设令牌格式为 "Bearer <token>"
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    None
}

fn token_from_query(req: &HttpRequest) -> Option<String> {
    let query_string = req.query_string();

    // 解析查询字符串
    let url = Url::parse(&format!("http://localhost/?{}", query_string)).ok()?;

    // 从查询参数中获取 token
    url.query_pairs()
        .find(|(key, _)| key == "token")
        .map(|(_, value)| value.to_string())
}

fn verify_jwt(
    state: &AppState,
    req: &HttpRequest,
    token_finders: &[TokenFinder],
) -> Result<UserClaims, JwtError> {
    let mut token_str = None;
    for &finder in token_finders {
        if let Some(token) = finder(&req) {
            token_str = Some(token);
            break;
        }
    }
    let token_str = token_str.ok_or(JwtError::NoTokenFound)?;
    let jwt_secret = state.app_cfg.jwt_secret();
    let jwt_expire_secs = state.app_cfg.jwt_expire_secs();
    let token_svc = JwtTokenService::new(&jwt_secret, jwt_expire_secs);

    let claims = token_svc.verify(&token_str)?;
    Ok(claims)
}

pub async fn jwt_refresher(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let state = req.app_data::<web::Data<AppState>>().unwrap();

    let mut token: String = String::new();
    if let Some(extracted_claims) = req.extensions().get::<UserClaims>().cloned() {
        let token_svc =
            JwtTokenService::new(&state.app_cfg.jwt_secret(), state.app_cfg.jwt_expire_secs());
        token = token_svc
            .issue(&extracted_claims)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    }

    let mut rsp = next.call(req).await?;
    rsp.headers_mut().insert(
        HeaderName::from_static(consts::UI_AUTHORIZATION_HEADER),
        HeaderValue::from_str(token.as_str()).unwrap(),
    );
    Ok(rsp)
}
