use std::future::Future;

use axum_core::extract::FromRequestParts;
use http::{request::Parts, StatusCode};

use crate::csrf::{error::Error, token::Token};

impl<S> FromRequestParts<S> for Token
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    fn from_request_parts(
        parts: &mut Parts,
        _: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            parts
                .extensions
                .get::<Token>()
                .cloned()
                .ok_or(Error::ExtensionNotFound("Token".into()))
                .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
        }
    }
}
