use axum::{response::IntoResponse, Json};
use axum_login::AuthSession;
use session::SessionBackend;

use crate::auth::error::Error;

pub mod error;
pub mod provider;
pub mod session;
pub mod user;

pub const CSRF_SESSION_KEY: &str = "csrf";

pub async fn me(auth_session: AuthSession<SessionBackend>) -> Result<impl IntoResponse, Error> {
    let Some(user) = auth_session.user else {
        return Err(Error::Unauthorized);
    };

    Ok(Json(user).into_response())
}
