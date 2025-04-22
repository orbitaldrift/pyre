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
use garde::Validate;
use serde::{Deserialize, Serialize};
use state::AppState;
use time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    decompression::DecompressionLayer,
    limit::RequestBodyLimitLayer,
    metrics::{in_flight_requests::InFlightRequestsCounter, InFlightRequestsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tower_sessions::cookie::Key;
use tower_sessions_redis_store::RedisStore;
use tracing::{info, Span};

use crate::{auth::session::SessionBackend, config::Config};

pub mod server;
pub mod state;

pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    #[garde(range(min = 1, max = 30))]
    pub session_days: i64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self { session_days: 7 }
    }
}

pub async fn router_with_middlewares(
    router: Router<AppState>,
    state: AppState,
    cfg: Config,
) -> color_eyre::Result<Router> {
    info!("creating middlewares");

    let session_store = RedisStore::new(state.redis_pool.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(
            cfg.session.session_days,
        )))
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_always_save(true)
        .with_signed(Key::from(state.secret.unsecure()));

    let backend = SessionBackend::new(Arc::new(state.clone()));
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    let middlewares = ServiceBuilder::new()
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(
            cfg.http.timeout,
        )))
        .layer(CatchPanicLayer::new())
        .layer(tower::limit::ConcurrencyLimitLayer::new(cfg.http.max_conns))
        .layer(InFlightRequestsLayer::new(InFlightRequestsCounter::new()))
        .layer(SetRequestIdLayer::new(X_REQUEST_ID, MakeRequestUuid))
        .layer(RequestBodyLimitLayer::new(cfg.http.max_body))
        .layer(TraceLayer::new_for_http().make_span_with(make_span))
        .layer(
            CorsLayer::new()
                .allow_origin(cfg.http.origins)
                .allow_methods(vec![
                    axum::http::Method::GET,
                    axum::http::Method::HEAD,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers(Any),
        )
        .layer(auth_layer)
        .layer(CompressionLayer::new())
        .layer(PropagateRequestIdLayer::new(X_REQUEST_ID))
        .layer(DecompressionLayer::new());

    Ok(router.layer(middlewares).with_state(state))
}

fn get_sensitive_headers() -> HashSet<HeaderName> {
    let mut sensitive = HashSet::new();
    sensitive.insert(HeaderName::from_static("cookie"));
    sensitive.insert(HeaderName::from_static("authorization"));
    sensitive.insert(HeaderName::from_static("proxy-authorization"));
    sensitive.insert(HeaderName::from_static("set-cookie"));
    sensitive
}

fn make_span<B>(request: &Request<B>) -> Span {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .expect("RequestId not found in request extensions, not set by middleware");

    let request_id = request_id
        .header_value()
        .to_str()
        .expect("request id header value not a string");

    let sensitive_headers = get_sensitive_headers();

    let mut redacted_headers = request.headers().clone();
    for name in &sensitive_headers {
        if redacted_headers.contains_key(name) {
            redacted_headers.insert(name, HeaderValue::from_static("[REDACTED]"));
        }
    }

    // Ellipse the request URI if it's too long
    let req_uri = {
        let uri = request.uri().to_string();
        if uri.len() > 100 {
            format!("{}...{}", &uri[..45], &uri[uri.len() - 45..])
        } else {
            uri
        }
    };

    tracing::info_span!(
        "request",
        id = %request_id,
        method = %request.method(),
        uri = %req_uri,
        version = ?request.version(),
        headers = ?redacted_headers,
    )
}
