use std::fmt::{Debug, Display};

use axum::response::{IntoResponse, Response};
use hyper::StatusCode;

pub trait AppError: Sized + Debug + Display {
    fn status_code(&self) -> StatusCode;

    fn into_response(self) -> Response {
        let status = self.status_code();

        match status.as_u16() {
            500..=599 => {
                tracing::error!(
                    counter.errors = 1,
                    status = %status,
                    source = %self,
                    "Server error"
                );
            }
            400..=499 => {
                tracing::warn!(
                    counter.warnings = 1,
                    status = %status,
                    source = %self,
                    "Client error"
                );
            }
            _ => {
                tracing::debug!(
                    status = %status,
                    source = %self,
                    "Other error"
                );
            }
        }

        let body = if status.is_server_error() {
            "Internal server error".to_string()
        } else {
            format!("{self}")
        };

        (status, body).into_response()
    }
}
