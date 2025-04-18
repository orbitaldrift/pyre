use http::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Maps the [`hmac::digest::InvalidLength`] error.
    #[error(transparent)]
    InvalidLength(#[from] hmac::digest::InvalidLength),
    /// Maps the [`http::header::InvalidHeaderValue`] error.
    #[error(transparent)]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
    /// An expected extension was missing.
    #[error("couldn't extract `{0}`. is `CsrfLayer` enabled?")]
    ExtensionNotFound(String),
    /// The token cookie couldn't be found by the name given.
    #[error("couldn't get cookie")]
    NoCookie,
}

impl Error {
    pub(crate) fn make_layer_error<T: Default>(err: impl std::error::Error) -> http::Response<T> {
        tracing::error!(err = %err);

        let mut response = http::Response::default();
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;

        response
    }

    pub(crate) fn make_layer_forbidden<T: Default>() -> http::Response<T> {
        let mut response = http::Response::default();
        *response.status_mut() = StatusCode::FORBIDDEN;
        response
    }
}
