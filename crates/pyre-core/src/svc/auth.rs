use std::str::FromStr;

use axum::{
    response::{IntoResponse, Redirect},
    Form,
};
use axum_login::AuthSession;
use hyper::HeaderMap;
use tracing::info;
use uuid::Uuid;

use super::Error;
use crate::auth::backend::{Backend, Credentials};

pub async fn login(
    header_map: HeaderMap,
    mut auth_session: AuthSession<Backend>,
    Form(creds): Form<Credentials>,
) -> Result<impl IntoResponse, crate::Error<Error>> {
    let id = header_map
        .get(String::from("x-request-id"))
        .unwrap()
        .to_str()
        .unwrap();

    info!("cred: {:?}", creds);

    let user = match auth_session.authenticate(creds.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err(crate::Error::new(Error::Unauthorized(
                Uuid::from_str(id).map_err(|_| crate::Error::new(Error::BadId))?,
            )))
        }
        Err(_) => {
            return Err(crate::Error::new(Error::Unauthorized(
                Uuid::from_str(id).map_err(|_| crate::Error::new(Error::BadId))?,
            )))
        }
    };

    if auth_session.login(&user).await.is_err() {
        return Err(crate::Error::new(Error::Unauthorized(
            Uuid::from_str(id).map_err(|_| crate::Error::new(Error::BadId))?,
        )));
    }

    Ok(Redirect::to("/protected").into_response())
}
