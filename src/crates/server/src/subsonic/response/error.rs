use actix_web::{HttpResponse, ResponseError};
use application::error::AppError;
use serde::Serialize;
use std::fmt;

use super::{JsonWrapper, Subsonic};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubsonicError {
    pub code: u16,

    pub message: String,
}

impl SubsonicError {
    pub fn new(code: u16, message: String) -> Self {
        Self { code, message }
    }
    pub fn wrap(mut self, message: String) -> Self {
        self.message.push_str(": ");
        self.message.push_str(&message);
        self
    }
}

macro_rules! rhythm_error {
    (
        $(
            ($num:expr, $konst:ident, $phrase:expr);
        )+
    ) => {
        impl SubsonicError {
        $(
            pub fn $konst() -> SubsonicError {
                SubsonicError { code: $num, message: String::from($phrase) }
            }
        )+
        }
    }
}

impl From<AppError> for SubsonicError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::InvalidInput(message) => {
                SubsonicError::error_missing_parameter().wrap(message)
            }
            AppError::AuthError(message) => {
                SubsonicError::error_authentication_fail().wrap(message)
            }
            AppError::AggregateNotFound(_, _) => SubsonicError::error_data_not_found(),
            _ => SubsonicError::error_generic().wrap(err.to_string()),
        }
    }
}

rhythm_error! {
    (0, error_generic, "A generic error");
    (10, error_missing_parameter, "Required parameter is missing");
    (20, error_client_too_old, "Incompatible Subsonic REST protocol version. Client must upgrade");
    (30, error_server_too_old, "Incompatible Subsonic REST protocol version. Server must upgrade");
    (40, error_authentication_fail, "Wrong username or password");
    (50, error_authorization_fail, "User is not authorized for the given operation");
    (60, error_trial_expired, "The trial period for the Subsonic server is over. Please upgrade to Subsonic Premium. Visit subsonic.org for details");
    (70, error_data_not_found, "The requested data was not found");
}

impl fmt::Display for SubsonicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubsonicError {}: {}", self.code, self.message)
    }
}

impl ResponseError for SubsonicError {
    fn error_response(&self) -> HttpResponse {
        // 将错误转换为 Subsonic 响应，并使用 JsonWrapper 包装
        let subsonic: Subsonic = SubsonicError::new(self.code, self.message.clone()).into();
        let wrapper = JsonWrapper {
            subsonic_response: subsonic,
        };
        HttpResponse::Ok().json(wrapper)
    }
}
