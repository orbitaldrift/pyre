use axum_core::extract::FromRequestParts;
use http::{request::Parts, StatusCode};

use crate::{cookie::CsrfCookie, error::Error};

impl<S> FromRequestParts<S> for CsrfCookie
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CsrfCookie>()
            .cloned()
            .ok_or(Error::ExtensionNotFound("CsrfToken".into()))
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
    }
}
