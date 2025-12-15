use application::auth::{TokenService, UserClaims};
use application::error::AppError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use bcrypt::hash as bcrypt_hash;
use bcrypt::verify as bcrypt_verify;
use chrono::Utc;
use hmac::{Hmac, Mac};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use sha2::Sha256;

use serde::{Deserialize, Serialize};

type HmacSha256 = Hmac<Sha256>;

pub trait AuthConfig {
    fn jwt_secret(&self) -> &str;
    fn jwt_expire_secs(&self) -> i64;
    fn salt_cost(&self) -> i32;
}

#[derive(Debug, Clone)]
pub struct BcryptPasswordHasher {
    salt_cost: i32,
}

impl BcryptPasswordHasher {
    pub fn new(salt_cost: i32) -> Self {
        Self { salt_cost }
    }
}

impl BcryptPasswordHasher {
    pub fn verify(&self, pwd: &str, hashed_pwd: &str) -> bool {
        bcrypt_verify(pwd, hashed_pwd).unwrap_or(false)
    }
    pub fn hash(&self, plain: &str) -> Result<String, bcrypt::BcryptError> {
        bcrypt_hash(plain, self.salt_cost as u32)
    }
}

impl application::auth::PasswordHasher for BcryptPasswordHasher {
    fn hash(&self, plain: &str) -> Result<String, AppError> {
        bcrypt_hash(plain, self.salt_cost as u32)
            .map_err(|e| AppError::AuthError(e.to_string()))
    }

    fn verify(&self, pwd: &str, hashed_pwd: &str) -> Result<(), AppError> {
        if bcrypt_verify(pwd, hashed_pwd).unwrap_or(false) {
            Ok(())
        } else {
            Err(AppError::AuthError("invalid password".to_string()))
        }
    }
}

#[derive(Debug, Clone)]
pub struct JwtTokenService {
    jwt_secret: String,
    exp_secs: i64,
}

impl JwtTokenService {
    pub fn new(jwt_secret: &str, exp_secs: i64) -> Self {
        Self {
            jwt_secret: jwt_secret.to_string(),
            exp_secs,
        }
    }
    fn encode_claims<T: Serialize>(&self, claims: &T) -> Result<String, AppError> {
        let key = EncodingKey::from_secret(self.jwt_secret.as_bytes());
        encode(&Header::new(Algorithm::HS256), claims, &key)
            .map_err(|e| AppError::AuthError(e.to_string()))
    }

    /// 使用 HS128 算法编码 claims（签名更短，适合 cover art token）
    fn encode_claims_short<T: Serialize>(&self, claims: &T) -> Result<String, AppError> {
        let key = EncodingKey::from_secret(self.jwt_secret.as_bytes());
        encode(&Header::new(Algorithm::HS384), claims, &key)
            .map_err(|e| AppError::AuthError(e.to_string()))
    }

    fn decode_claims<T: for<'de> Deserialize<'de>>(&self, token: &str) -> Result<T, AppError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<T>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| AppError::AuthError(e.to_string()))?;

        Ok(token_data.claims)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    pub sub: String,
    pub adm: bool,
    pub exp: i64,
    pub iat: i64,
}

/// 简化的 cover art token claims，只包含必要字段，使用短字段名以减小 token 大小
#[derive(Debug, Serialize, Deserialize)]
struct CoverArtClaims {
    #[serde(rename = "s")]
    pub sub: String, // cover_art_id
    #[serde(rename = "e")]
    pub exp: i64, // 过期时间
}

impl JwtClaims {
    fn new(claims: &UserClaims, exp_secs: i64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            sub: claims.user_name.clone(),
            adm: claims.is_admin,
            exp: now + exp_secs,
            iat: now,
        }
    }
    fn new_with_sub(custom_sub: &str, exp_secs: i64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            sub: custom_sub.to_string(),
            adm: false,
            exp: now + exp_secs,
            iat: now,
        }
    }
}

impl From<JwtClaims> for UserClaims {
    fn from(claims: JwtClaims) -> Self {
        Self {
            user_name: claims.sub,
            is_admin: claims.adm,
        }
    }
}
impl TokenService for JwtTokenService {
    fn issue(&self, claims: &UserClaims) -> Result<String, AppError> {
        let claims: JwtClaims = JwtClaims::new(claims, self.exp_secs);
        let key = EncodingKey::from_secret(self.jwt_secret.as_bytes());
        let header = Header::new(Algorithm::HS256);
        let token =
            encode(&header, &claims, &key).map_err(|e| AppError::AuthError(e.to_string()))?;
        Ok(token)
    }

    fn verify(&self, token: &str) -> Result<UserClaims, AppError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let token_data = decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| AppError::AuthError(e.to_string()))?;

        Ok(token_data.claims.into())
    }
}

impl JwtTokenService {
    pub fn issue_sub(&self, custom_sub: &str) -> Result<String, AppError> {
        let claims = JwtClaims::new_with_sub(custom_sub, self.exp_secs);
        self.encode_claims(&claims)
    }
    pub fn verify_sub(&self, token: &str) -> Result<String, AppError> {
        let token_data = self
            .decode_claims::<JwtClaims>(token)
            .map_err(|e| AppError::AuthError(e.to_string()))?;
        Ok(token_data.sub)
    }

    /// 生成超短的 cover art token（自定义格式，比 JWT 更紧凑）
    /// 格式: base64(cover_art_id|exp) + "." + base64(hmac_signature)
    /// 总长度约 30-40 字符（相比 JWT 的 70-80 字符减少约 50%）
    pub fn issue_cover_art_token_short(&self, cover_art_id: String) -> Result<String, AppError> {
        let exp = Utc::now().timestamp() + self.exp_secs;

        // 构建 payload: cover_art_id|exp
        let payload = format!("{}|{}", cover_art_id, exp);
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload.as_bytes());

        // 计算 HMAC-SHA256 签名
        let mut mac = HmacSha256::new_from_slice(self.jwt_secret.as_bytes())
            .map_err(|e| AppError::AuthError(format!("Invalid key: {}", e)))?;
        mac.update(payload.as_bytes());
        let signature = mac.finalize();
        let signature_bytes = signature.into_bytes();

        // 只使用前 16 字节的签名（128位，足够安全且更短）
        let short_signature = &signature_bytes[..16];
        let sig_b64 = URL_SAFE_NO_PAD.encode(short_signature);

        // 组合: payload.signature
        Ok(format!("{}.{}", payload_b64, sig_b64))
    }

    /// 验证超短的 cover art token
    pub fn verify_cover_art_token(&self, token: &str) -> Result<String, AppError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 2 {
            return Err(AppError::AuthError("Invalid token format".to_string()));
        }

        // 解码 payload
        let payload_bytes = URL_SAFE_NO_PAD
            .decode(parts[0])
            .map_err(|e| AppError::AuthError(format!("Invalid payload: {}", e)))?;
        let payload = String::from_utf8(payload_bytes)
            .map_err(|e| AppError::AuthError(format!("Invalid payload encoding: {}", e)))?;

        // 解析 cover_art_id 和 exp
        let parts_payload: Vec<&str> = payload.split('|').collect();
        if parts_payload.len() != 2 {
            return Err(AppError::AuthError("Invalid payload format".to_string()));
        }

        let cover_art_id = parts_payload[0];
        let exp: i64 = parts_payload[1]
            .parse()
            .map_err(|e| AppError::AuthError(format!("Invalid exp: {}", e)))?;

        // 检查过期时间
        let now = Utc::now().timestamp();
        if exp < now {
            return Err(AppError::AuthError("Token expired".to_string()));
        }

        // 验证签名
        let mut mac = HmacSha256::new_from_slice(self.jwt_secret.as_bytes())
            .map_err(|e| AppError::AuthError(format!("Invalid key: {}", e)))?;
        mac.update(payload.as_bytes());
        let signature = mac.finalize();
        let signature_bytes = signature.into_bytes();
        let short_signature = &signature_bytes[..16];
        let expected_sig_b64 = URL_SAFE_NO_PAD.encode(short_signature);

        let provided_sig_b64 = parts[1];
        if expected_sig_b64 != provided_sig_b64 {
            return Err(AppError::AuthError("Invalid signature".to_string()));
        }

        Ok(cover_art_id.to_string())
    }
}

// 实现 CoverArtTokenService trait，让 JwtTokenService 可以直接用于应用服务层
impl application::query::shared::CoverArtTokenService for JwtTokenService {
    fn issue_cover_art_token(&self, cover_art_id: String) -> Result<String, AppError> {
        // 使用简化的 token 格式以减小大小
        self.issue_cover_art_token_short(cover_art_id)
    }
}
