use axum_core::extract::FromRequestParts;
use http::{request::Parts, StatusCode};

use crate::{error::Error, token::CsrfToken};

impl<S> FromRequestParts<S> for CsrfToken
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CsrfToken>()
            .cloned()
            .ok_or(Error::ExtensionNotFound("CsrfToken".into()))
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
    }
}
