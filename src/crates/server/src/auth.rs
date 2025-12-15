use actix_web::{web, HttpRequest, HttpResponse, Scope};
use application::auth::AuthService;
use infra::auth::{AuthConfig, BcryptPasswordHasher, JwtTokenService};
use infra::repository::postgres::command::user::UserRepositoryImpl;
use infra::Aes256GcmEncryptor;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::AppState;

/// Rate limiter for login attempts by IP
struct RateLimiter {
    attempts: HashMap<String, Vec<Instant>>,
    max_attempts: usize,
    window: Duration,
}

impl RateLimiter {
    fn new(max_attempts: usize, window_secs: u64) -> Self {
        Self {
            attempts: HashMap::new(),
            max_attempts,
            window: Duration::from_secs(window_secs),
        }
    }

    fn is_allowed(&mut self, ip: &str) -> bool {
        let now = Instant::now();
        let attempts = self.attempts.entry(ip.to_string()).or_default();

        // Remove expired attempts
        attempts.retain(|t| now.duration_since(*t) < self.window);

        if attempts.len() >= self.max_attempts {
            false
        } else {
            attempts.push(now);
            true
        }
    }

    #[allow(dead_code)]
    fn cleanup(&mut self) {
        let now = Instant::now();
        self.attempts.retain(|_, attempts| {
            attempts.retain(|t| now.duration_since(*t) < self.window);
            !attempts.is_empty()
        });
    }
}

static LOGIN_LIMITER: Lazy<Mutex<RateLimiter>> =
    Lazy::new(|| Mutex::new(RateLimiter::new(3, 60))); // 3 attempts per 60 seconds

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

fn get_client_ip(req: &HttpRequest) -> String {
    // Try X-Forwarded-For header first (for reverse proxy)
    if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
        if let Ok(s) = forwarded.to_str() {
            if let Some(ip) = s.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = req.headers().get("X-Real-IP") {
        if let Ok(s) = real_ip.to_str() {
            return s.trim().to_string();
        }
    }

    // Fall back to peer address
    req.peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub async fn login(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> HttpResponse {
    let client_ip = get_client_ip(&req);

    // Check rate limit
    {
        let mut limiter = LOGIN_LIMITER.lock();
        if !limiter.is_allowed(&client_ip) {
            return HttpResponse::TooManyRequests().json(ErrorResponse {
                error: "Too many login attempts. Please try again later.".to_string(),
            });
        }
    }

    let user_repo: Arc<dyn domain::user::UserRepository> =
        Arc::new(UserRepositoryImpl::new(state.db.clone()));
    let hasher: Arc<dyn application::auth::PasswordHasher> =
        Arc::new(BcryptPasswordHasher::new(10));
    let encryptor: Arc<dyn application::auth::PasswordEncryptor> = Arc::new(
        Aes256GcmEncryptor::new(&state.app_cfg.password_encryption_key())
            .expect("Failed to create password encryptor"),
    );
    let token_svc: Arc<dyn application::auth::TokenService> = Arc::new(JwtTokenService::new(
        &state.app_cfg.jwt_secret(),
        state.app_cfg.jwt_expire_secs(),
    ));

    let auth_service = AuthService::new(
        user_repo,
        hasher,
        encryptor,
        token_svc,
        state.id_generator.clone(),
    );

    match auth_service.login(&body.username, &body.password).await {
        Ok(token) => HttpResponse::Ok().json(LoginResponse { token }),
        Err(e) => HttpResponse::Unauthorized().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}

pub fn configure_service() -> Scope {
    web::scope("/auth").route("/login", web::post().to(login))
}
