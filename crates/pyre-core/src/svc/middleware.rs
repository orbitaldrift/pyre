use std::sync::Arc;

use axum::{extract::Request, http::HeaderName, Router};
use axum_login::{
    tower_sessions::{Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
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

use super::state::AppState;
use crate::{auth::session::SessionBackend, config::Config};

pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

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
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(move |req: &Request<_>| make_span(req, &cfg.telemetry.level)),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
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

fn make_span<B>(request: &Request<B>, level: &str) -> Span {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .expect("RequestId not found in request extensions, not set by middleware");

    let request_id = request_id
        .header_value()
        .to_str()
        .expect("request id header value not a string");

    // Ellipse the request URI path if it's too long
    let req_uri = {
        let uri = request.uri().path().to_string();
        if uri.len() > 30 {
            format!("{}...{}", &uri[..10], &uri[uri.len() - 10..])
        } else {
            uri
        }
    };

    match level {
        "trace" | "debug" => {
            tracing::debug_span!(
                "request",
                id = %request_id,
                method = %request.method(),
                uri = %req_uri,
                version = ?request.version(),
                headers = ?request.headers(),
            )
        }
        _ => {
            tracing::info_span!(
                "request",
                id = %request_id,
                method = %request.method(),
                uri = %req_uri,
                version = ?request.version(),
            )
        }
    }
}
