use axum::response::IntoResponse;

use super::session::SessionBackend;
use crate::error::AppError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] crate::db::Error),
    #[error(transparent)]
    Session(#[from] axum_login::tower_sessions::session::Error),

    #[error("invalid oauth csrf")]
    InvalidOAuthCsrf,

    #[error("failed to get oauth2 client: {0}")]
    ProviderNotFound(String),
    #[error("invalid oauth token request: {0}")]
    OAuth2TokenRequest(String),
    #[error("failed to fetch discord token: {0}")]
    DiscordToken(String),
    #[error("failed to parse discord body: {0}")]
    DiscordBody(String),
    #[error("failed to validate provider: {0}")]
    InvalidProvider(String),

    #[error("existing user attempted to login with wrong provider")]
    UserExists,

    #[error("unauthorized")]
    Unauthorized,
}

impl From<axum_login::Error<SessionBackend>> for Error {
    fn from(e: axum_login::Error<SessionBackend>) -> Self {
        match e {
            axum_login::Error::Session(e) => Error::Session(e),
            axum_login::Error::Backend(e) => e,
        }
    }
}

impl AppError for Error {
    fn status_code(&self) -> hyper::StatusCode {
        match self {
            Error::OAuth2TokenRequest(_)
            | Error::DiscordToken(_)
            | Error::DiscordBody(_)
            | Error::InvalidProvider(_)
            | Error::ProviderNotFound(_)
            | Error::InvalidOAuthCsrf
            | Error::UserExists => hyper::StatusCode::BAD_REQUEST,
            Error::Unauthorized => hyper::StatusCode::UNAUTHORIZED,
            Error::Session(_) | Error::Database(_) => hyper::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        <Self as AppError>::into_response(self)
    }
}
