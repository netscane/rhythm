use crate::{consts, AppState};
use actix_cors::Cors;
use application::auth::{PasswordEncryptor, UserClaims};
use domain::user::UserError;
use infra::repository::postgres::command::user::UserRepositoryImpl;
use infra::Aes256GcmEncryptor;
use log::{info, warn};

use actix_web::{
    body::MessageBody,
    cookie::Cookie,
    dev::{ServiceRequest, ServiceResponse},
    http::header::HeaderName,
    middleware::Next,
    web, HttpMessage,
};
use url::Url;

pub async fn auth_header_mapper(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    // pre-processing
    if let Some(auth_header) = req.headers().get(consts::UI_AUTHORIZATION_HEADER) {
        let auth_header_cloned = auth_header.clone();
        req.headers_mut().insert(
            HeaderName::from_bytes(b"Authorization").unwrap(),
            auth_header_cloned,
        );
    };
    next.call(req).await
    // post-processing
}

#[derive(Clone)]
pub struct ClientUniqueID(pub String);

/// client_unique_id middleware sets a unique client ID as a cookie if it's provided in the request header.
/// If the unique client ID is not in the header but present as a cookie, it adds the ID to the request context.
pub async fn client_unique_id(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    // 从 header 中获取 client unique ID
    let header_client_id = req
        .headers()
        .get(consts::UI_CLIENT_UNIQUE_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let mut client_unique_id: Option<String> = None;

    // 如果 header 中有 client unique ID，优先使用它
    if let Some(ref id) = header_client_id {
        client_unique_id = Some(id.clone());
    } else {
        // 如果 header 中没有，从 cookie 中读取
        if let Some(cookie) = req.cookie(consts::UI_CLIENT_UNIQUE_ID_HEADER) {
            client_unique_id = Some(cookie.value().to_string());
        }
    }

    // 如果找到了有效的 client unique ID，将其添加到 request extensions
    if let Some(ref id) = client_unique_id {
        req.extensions_mut().insert(ClientUniqueID(id.clone()));
    }

    // 获取 base path（从配置中获取，如果没有则使用 "/"）
    // 在调用 next 之前准备好所有需要的信息
    let base_path: String = req
        .app_data::<web::Data<AppState>>()
        .and_then(|state| {
            // 从 base_url 中提取路径部分，如果没有路径则使用 "/"
            let base_url = state.app_cfg.base_url();
            Url::parse(&base_url)
                .ok()
                .and_then(|url| {
                    let path = url.path();
                    if path.is_empty() || path == "/" {
                        Some("/".to_string())
                    } else {
                        Some(path.to_string())
                    }
                })
                .or(Some("/".to_string()))
        })
        .unwrap_or_else(|| "/".to_string());

    // 如果 header 中有 client unique ID，准备设置 cookie
    let cookie_to_set = if let Some(id) = &header_client_id {
        // 创建 cookie，设置 HttpOnly, Secure, SameSite=Strict
        let mut cookie = Cookie::new(consts::UI_CLIENT_UNIQUE_ID_HEADER, id.clone());
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(actix_web::cookie::SameSite::Strict);
        cookie.set_path(&base_path);
        cookie.set_max_age(actix_web::cookie::time::Duration::seconds(
            consts::COOKIE_EXPIRY,
        ));
        Some(cookie)
    } else {
        None
    };

    let mut rsp = next.call(req).await?;

    // 如果准备了 cookie，设置它
    if let Some(cookie) = cookie_to_set {
        rsp.response_mut().add_cookie(&cookie)?;
    }

    Ok(rsp)
}

pub fn cors() -> Cors {
    Cors::default()
        .allow_any_origin()
        .allowed_methods(vec!["GET", "POST", "PATCH", "PUT", "DELETE", "HEAD"])
        .allow_any_header()
        .max_age(3600)
}

type UsernameFinder = fn(&ServiceRequest) -> Option<Username>;

#[derive(Clone)]
struct Username(String);

fn username_from_config(req: &ServiceRequest) -> Option<Username> {
    let state = req.app_data::<web::Data<AppState>>()?;
    let cfg_str = state.app_cfg.auto_login_username.read().unwrap().clone();
    (!cfg_str.is_empty()).then(|| Username(cfg_str))
}

fn username_from_token(req: &ServiceRequest) -> Option<Username> {
    req.extensions()
        .get::<UserClaims>()
        .map(|claims| Username(claims.user_name.clone()))
}

fn username_from_subsonic_params(req: &ServiceRequest) -> Option<Username> {
    // 检查是否是 Subsonic API 请求
    if !req.path().starts_with("/rest/") {
        return None;
    }

    // 从查询参数中获取用户名
    let query = req.query_string();
    for param in query.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            if key == "u" {
                return Some(Username(value.to_string()));
            }
        }
    }
    None
}

#[derive(Clone)]
pub struct RequestUsername(pub String);

#[derive(Clone)]
pub struct RequestClient(pub String);

#[derive(Clone)]
pub struct RequestVersion(pub String);

/// Get username from reverse proxy header (X-Forwarded-User or similar)
fn username_from_reverse_proxy_header(req: &ServiceRequest) -> Option<String> {
    // Try common reverse proxy headers
    let headers_to_try = ["X-Forwarded-User", "X-Remote-User", "X-User", "Remote-User"];

    for header_name in &headers_to_try {
        if let Some(header_value) = req.headers().get(*header_name) {
            if let Ok(username) = header_value.to_str() {
                if !username.is_empty() {
                    return Some(username.to_string());
                }
            }
        }
    }
    None
}

/// Parse query string and extract parameter value
fn get_query_param(query_string: &str, param_name: &str) -> Option<String> {
    let url = format!("http://localhost/?{}", query_string);
    Url::parse(&url)
        .ok()?
        .query_pairs()
        .find(|(key, _)| key == param_name)
        .map(|(_, value)| value.to_string())
}

/// check_required_parameters middleware checks for required query parameters.
/// If username is found in reverse proxy header, only "v" and "c" are required.
/// Otherwise, "u", "v", and "c" are required.
pub async fn check_required_parameters(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    use crate::subsonic::response::error::SubsonicError;
    //info!("check_required_parameters: {:?}", req);

    // Try to get username from reverse proxy header
    let username_from_header = username_from_reverse_proxy_header(&req);

    // Determine required parameters based on whether username is in header
    let required_parameters = if username_from_header.is_some() {
        vec!["v", "c"]
    } else {
        vec!["u", "v", "c"]
    };

    // Get query string
    let query_string = req.query_string();

    // Check all required parameters
    for param in &required_parameters {
        if get_query_param(query_string, param).is_none() {
            let error_msg = format!("Required parameter '{}' is missing", param);
            warn!("{}: {}", req.path(), error_msg);
            let error = SubsonicError::error_missing_parameter().wrap(error_msg);
            return Err(actix_web::error::ErrorBadRequest(error));
        }
    }

    // Extract parameter values
    let mut username = username_from_header.unwrap_or_default();
    if username.is_empty() {
        username = get_query_param(query_string, "u").unwrap_or_default();
    }
    let client = get_query_param(query_string, "c").unwrap_or_default();
    let version = get_query_param(query_string, "v").unwrap_or_default();

    // Log request information (similar to Go code) - before moving values
    log::debug!(
        "API: New request {} - username: {}, client: {}, version: {}",
        req.path(),
        username,
        client,
        version
    );

    // Add values to request extensions
    if !username.is_empty() {
        req.extensions_mut()
            .insert(RequestUsername(username.clone()));
    }
    if !client.is_empty() {
        req.extensions_mut().insert(RequestClient(client));
    }
    if !version.is_empty() {
        req.extensions_mut().insert(RequestVersion(version));
    }

    next.call(req).await
}

pub async fn authenticator(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    println!("authenticator");

    let username_finders: &[UsernameFinder] = &[
        username_from_config,
        username_from_token,
        username_from_subsonic_params,
    ];
    let username = username_finders.iter().find_map(|finder| finder(&req));

    let Some(username) = username else {
        return Err(actix_web::error::ErrorUnauthorized("Unauthorized"));
    };

    req.extensions_mut().insert(username.clone());

    let state = req
        .app_data::<web::Data<AppState>>()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Missing AppState"))?;

    let repo = UserRepositoryImpl::new(state.db.clone());
    let user = repo
        .find_by_username(&username.0)
        .await
        .map_err(|e| match e {
            UserError::InvalidUserOrPassword(_) => {
                actix_web::error::ErrorUnauthorized("Unauthorized")
            }
            _ => actix_web::error::ErrorInternalServerError(e.to_string()),
        })?
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("User not found"))?;

    req.extensions_mut().insert(user);
    next.call(req).await
}

/// Subsonic API authentication middleware
/// Supports:
/// 1. Token authentication: t=token&s=salt where token = md5(password + salt)
/// 2. Plain password: p=password
/// 3. Hex-encoded password: p=enc:hexEncodedPassword
/// 4. Reverse proxy header authentication
pub async fn subsonic_authenticator(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    use crate::subsonic::response::error::SubsonicError;

    let query_string = req.query_string().to_string();

    let state = req
        .app_data::<web::Data<AppState>>()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Missing AppState"))?
        .clone();

    // Try reverse proxy header first
    if let Some(username) = username_from_reverse_proxy_header(&req) {
        let repo = UserRepositoryImpl::new(state.db.clone());
        let user = repo
            .find_by_username(&username)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
            .ok_or_else(|| actix_web::error::ErrorUnauthorized("User not found"))?;
        req.extensions_mut().insert(user);
        return next.call(req).await;
    }

    // Try auto login from config
    if let Some(Username(username)) = username_from_config(&req) {
        let repo = UserRepositoryImpl::new(state.db.clone());
        let user = repo
            .find_by_username(&username)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
            .ok_or_else(|| actix_web::error::ErrorUnauthorized("User not found"))?;
        req.extensions_mut().insert(user);
        return next.call(req).await;
    }

    // Get username from query params
    let username = get_query_param(&query_string, "u").ok_or_else(|| {
        let error =
            SubsonicError::error_missing_parameter().wrap("Missing parameter 'u'".to_string());
        actix_web::error::ErrorBadRequest(error)
    })?;

    let repo = UserRepositoryImpl::new(state.db.clone());
    let user = repo
        .find_by_username(&username)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| {
            let error =
                SubsonicError::error_authentication_fail().wrap("User not found".to_string());
            actix_web::error::ErrorUnauthorized(error)
        })?;

    // Try token authentication first (t + s)
    // Subsonic token: t = md5(password + s) where password is the original plain password
    if let (Some(token), Some(salt)) = (
        get_query_param(&query_string, "t"),
        get_query_param(&query_string, "s"),
    ) {
        // Decrypt the stored encrypted password to get the original password
        let encryptor = match Aes256GcmEncryptor::new(&state.app_cfg.password_encryption_key()) {
            Ok(e) => e,
            Err(e) => {
                let error = SubsonicError::error_generic().wrap(format!("Encryption error: {}", e));
                return Err(actix_web::error::ErrorInternalServerError(error));
            }
        };

        let plain_password = match encryptor.decrypt(&user.encrypted_password) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    "Failed to decrypt password for user {}: {}",
                    user.username, e
                );
                let error =
                    SubsonicError::error_authentication_fail().wrap("Invalid token".to_string());
                return Err(actix_web::error::ErrorUnauthorized(error));
            }
        };

        // token = md5(password + salt)
        let expected_token = format!("{:x}", md5::compute(format!("{}{}", plain_password, salt)));

        if token.to_lowercase() == expected_token.to_lowercase() {
            req.extensions_mut().insert(user);
            return next.call(req).await;
        }

        let error = SubsonicError::error_authentication_fail().wrap("Invalid token".to_string());
        return Err(actix_web::error::ErrorUnauthorized(error));
    }

    // Try password authentication (p)
    // For Subsonic: p = password (plain or enc:hex)
    if let Some(password) = get_query_param(&query_string, "p") {
        info!("Password: {}", password);
        let plain_password = decode_subsonic_password(&password);
        info!("Plain password: {}", plain_password);

        // Decrypt the stored encrypted password and compare
        let encryptor = match Aes256GcmEncryptor::new(&state.app_cfg.password_encryption_key()) {
            Ok(e) => e,
            Err(e) => {
                let error = SubsonicError::error_generic().wrap(format!("Encryption error: {}", e));
                return Err(actix_web::error::ErrorInternalServerError(error));
            }
        };

        let stored_password = match encryptor.decrypt(&user.encrypted_password) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    "Failed to decrypt password for user {}: {}",
                    user.username, e
                );
                let error =
                    SubsonicError::error_authentication_fail().wrap("Invalid password".to_string());
                return Err(actix_web::error::ErrorUnauthorized(error));
            }
        };

        // Compare passwords directly
        if plain_password == stored_password {
            req.extensions_mut().insert(user);
            return next.call(req).await;
        }

        let error = SubsonicError::error_authentication_fail().wrap("Invalid password".to_string());
        return Err(actix_web::error::ErrorUnauthorized(error));
    }

    // No valid authentication method found
    let error = SubsonicError::error_missing_parameter()
        .wrap("Missing authentication parameters (p or t+s)".to_string());
    Err(actix_web::error::ErrorBadRequest(error))
}

/// Decode Subsonic password parameter
/// Supports: plain text, or enc:hexEncodedPassword
fn decode_subsonic_password(password: &str) -> String {
    if let Some(hex_encoded) = password.strip_prefix("enc:") {
        // Decode hex-encoded password
        if let Ok(bytes) = hex::decode(hex_encoded) {
            if let Ok(decoded) = String::from_utf8(bytes) {
                return decoded;
            }
        }
    }
    password.to_string()
}
