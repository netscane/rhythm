use crate::consts;
use actix_web::{FromRequest, HttpRequest};
use serde::de::DeserializeOwned;
use std::future::{ready, Ready};
use std::ops::Deref;

pub fn image_url(base_url: &str, cover_art_token: &str, size: i32) -> String {
    format!(
        "{}/{}?size={}",
        absolute_url(base_url, consts::URL_PATH_PUBLIC_IMAGES),
        cover_art_token,
        size
    )
}

pub struct ImageUrls {
    pub small: String,
    pub medium: String,
    pub large: String,
}

/// 生成三种尺寸的图片 URL（接口层辅助函数）
/// cover_art_token 由应用服务层生成
pub fn generate_image_urls(base_url: &str, cover_art_token: &str) -> ImageUrls {
    ImageUrls {
        small: image_url(base_url, cover_art_token, 300),
        medium: image_url(base_url, cover_art_token, 600),
        large: image_url(base_url, cover_art_token, 1200),
    }
}

pub fn absolute_url(base_url: &str, path: &str) -> String {
    format!("{}{}", base_url, path)
}

/// 自定义 Query 提取器，支持数组索引格式（如 songIdToAdd[0]=xxx）
/// 使用 serde_qs 代替 serde_urlencoded
#[derive(Debug, Clone)]
pub struct QsQuery<T>(pub T);

impl<T> Deref for QsQuery<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> FromRequest for QsQuery<T>
where
    T: DeserializeOwned,
{
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let query_string = req.query_string();
        let config = serde_qs::Config::new(5, false);
        match config.deserialize_str::<T>(query_string) {
            Ok(value) => ready(Ok(QsQuery(value))),
            Err(e) => ready(Err(actix_web::error::ErrorBadRequest(format!(
                "Query string parse error: {}",
                e
            )))),
        }
    }
}
