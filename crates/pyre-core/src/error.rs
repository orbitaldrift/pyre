use std::fmt::{Debug, Display};

use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use uuid::Uuid;

pub trait AppError: Debug + Display {
    fn id(&self) -> Uuid;
    fn status_code(&self) -> StatusCode;
}

#[derive(Debug)]
pub struct Error<T: AppError> {
    pub source: T,
}

impl<T> IntoResponse for Error<T>
where
    T: AppError,
{
    fn into_response(self) -> Response {
        let status = self.source.status_code();
        let id = self.source.id();

        match status.as_u16() {
            500..=599 => {
                tracing::error!(
                    counter.errors = 1,
                    id = %id,
                    status = %status,
                    source = %self.source,
                    "Server error"
                );
            }
            400..=499 => {
                tracing::warn!(
                    counter.warnings = 1,
                    id = %id,
                    status = %status,
                    source = %self.source,
                    "Client error"
                );
            }
            _ => {
                tracing::debug!(
                    id = %id,
                    status = %status,
                    source = %self.source,
                    "Other error"
                );
            }
        }

        let body = if status.is_server_error() { "Internal server error".to_string() } else { format!("{}", self.source) };

        (status, body).into_response()
    }
}
