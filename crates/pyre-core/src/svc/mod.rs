use std::{collections::HashSet, sync::Arc};

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    Router,
};
use axum_login::{
    tower_sessions::{Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use pyre_axum_csrf::CsrfLayer;
use state::AppState;
use time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    add_extension::AddExtensionLayer,
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    decompression::DecompressionLayer,
    limit::RequestBodyLimitLayer,
    metrics::{in_flight_requests::InFlightRequestsCounter, InFlightRequestsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::{DefaultOnFailure, TraceLayer},
};
use tower_sessions::cookie::Key;
use tower_sessions_redis_store::RedisStore;
use tracing::{info, Level, Span};
use uuid::Uuid;

use crate::{auth::backend::Backend, config::Config, error::AppError};

pub mod auth;
pub mod server;
pub mod state;

pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unauth")]
    Unauthorized(Uuid),
    #[error("err")]
    InternalErr(Uuid),

    #[error("bad request id")]
    BadId,
}

impl AppError for Error {
    fn id(&self) -> Uuid {
        match self {
            Error::InternalErr(uuid) | Error::Unauthorized(uuid) => *uuid,
            Error::BadId => Uuid::new_v4(),
        }
    }

    fn status_code(&self) -> hyper::StatusCode {
        match self {
            Error::InternalErr(_) => hyper::StatusCode::INTERNAL_SERVER_ERROR,
            Error::Unauthorized(_) => hyper::StatusCode::UNAUTHORIZED,
            Error::BadId => hyper::StatusCode::BAD_REQUEST,
        }
    }
}

pub async fn add_middlewares(
    state: Arc<AppState>,
    router: Router,
    cfg: Config,
    secret: [u8; 64],
) -> color_eyre::Result<Router> {
    info!("creating middlewares");

    let session_store = RedisStore::new(state.redis_pool.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(
            cfg.session.session_days,
        )))
        .with_signed(Key::from(&secret));

    let backend = Backend::default();
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    let middlewares = ServiceBuilder::new()
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(
            cfg.http.timeout,
        )))
        .layer(CatchPanicLayer::new())
        .layer(tower::limit::ConcurrencyLimitLayer::new(cfg.http.max_conns))
        .layer(InFlightRequestsLayer::new(InFlightRequestsCounter::new()))
        .layer(SetRequestIdLayer::new(X_REQUEST_ID, MakeRequestUuid))
        .layer(AddExtensionLayer::new(state))
        .layer(RequestBodyLimitLayer::new(cfg.http.max_body))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_span_with_redacted_headers)
                .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(cfg.http.origins)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(auth_layer)
        .layer(CsrfLayer::new(secret.to_vec()))
        .layer(CompressionLayer::new())
        .layer(PropagateRequestIdLayer::new(X_REQUEST_ID))
        .layer(DecompressionLayer::new());

    Ok(router.layer(middlewares))
}

fn get_sensitive_headers() -> HashSet<HeaderName> {
    let mut sensitive = HashSet::new();
    sensitive.insert(HeaderName::from_static("cookie"));
    sensitive.insert(HeaderName::from_static("authorization"));
    sensitive.insert(HeaderName::from_static("proxy-authorization"));
    sensitive.insert(HeaderName::from_static("set-cookie"));
    sensitive
}

// Create a custom make_span function that redacts sensitive headers
fn make_span_with_redacted_headers<B>(request: &Request<B>) -> Span {
    let sensitive_headers = get_sensitive_headers();

    // Create a version of the headers with sensitive values redacted
    let mut redacted_headers = request.headers().clone();
    for name in &sensitive_headers {
        if redacted_headers.contains_key(name) {
            redacted_headers.insert(name, HeaderValue::from_static("[REDACTED]"));
        }
    }

    tracing::info_span!(
        "request",
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
        headers = ?redacted_headers,
    )
}
