use axum::{
    response::{IntoResponse, Redirect},
    Form,
};
use axum_login::AuthSession;
use session::{Credentials, SessionBackend};

use crate::auth::error::Error;

pub mod error;
pub mod provider;
pub mod session;
pub mod user;

pub async fn login(
    mut auth_session: AuthSession<SessionBackend>,
    Form(creds): Form<Credentials>,
) -> Result<impl IntoResponse, Error> {
    let Ok(Some(user)) = auth_session.authenticate(creds.clone()).await else {
        return Err(Error::Unauthorized);
    };

    if auth_session.login(&user).await.is_err() {
        return Err(Error::Unauthorized);
    }

    Ok(Redirect::to("/protected").into_response())
}
