use std::{path::PathBuf, process::exit, sync::Arc, time::Duration};

use axum::{http::HeaderName, routing::get};
use color_eyre::eyre::Context;
use config::Config;
use csrf::CsrfLayer;
use pyre_build::build_info;
use pyre_cli::shutdown::Shutdown;
use pyre_crypto::PkiCert;
use pyre_fs::toml::FromToml;
use pyre_telemetry::{Info, Telemetry};
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
    trace::{DefaultMakeSpan, DefaultOnFailure, TraceLayer},
};
use tracing::{error, Level};
use uuid::Uuid;

mod config;
mod csrf;
mod error;
mod server;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

async fn root() -> &'static str {
    "Hello, World from axum!"
}

struct State {}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> color_eyre::Result<()> {
    // Auth Layer PGSQL-SQLX + Sesssion RedisStore + Integrate with CSRF - Discord

    // DB
    // Graphql
    // Pooling, retries
    // Strong input validation around invariants
    // MTG card scheduled download, versioning
    // Cache layer for pg, card's data
    // WebTransport for any streaming
    // grafana tanka, ingress, grafana stack, terraform k8s cluster
    // k8s cluster pod resource monitoring
    // e2e tests, unit tests

    build_info!();
    color_eyre::install()?;
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cfg;
    {
        let _g = Telemetry::stdout();

        (cfg, _) = Config::from_toml_path::<PathBuf>(None)
            .await
            .unwrap_or_else(|e| {
                error!(%e, "failed to load config");
                exit(1)
            });

        Telemetry::new(
            &cfg.telemetry,
            Info {
                id: Uuid::new_v4().into(),
                domain: "odrift".to_string(),
                meta: None,
            },
        )
        .init()
        .inspect_err(|e| {
            error!(%e, "failed to initialize telemetry");
            exit(1)
        })
        .expect("failed to initialize telemetry");
    }

    let dir = std::env::current_dir().unwrap();
    println!("Current directory: {:?}", dir);
    tokio::fs::read(cfg.server.cert.clone()).await.unwrap();

    let shutdown = Shutdown::new_with_all_signals().install();
    let pki = PkiCert::from_bytes(
        tokio::fs::read(&cfg.server.cert)
            .await
            .context("cert not found")?,
        tokio::fs::read(&cfg.server.key)
            .await
            .context("key not found")?,
    )?;
    let x_request_id = HeaderName::from_static("x-request-id");

    let state = State {};

    let csrf_secret = "secret".as_bytes().to_vec();

    let middlewares = ServiceBuilder::new()
        .layer(TimeoutLayer::new(Duration::from_secs(cfg.server.timeout)))
        .layer(CatchPanicLayer::new())
        .layer(tower::limit::ConcurrencyLimitLayer::new(
            cfg.server.max_conns,
        ))
        .layer(InFlightRequestsLayer::new(InFlightRequestsCounter::new()))
        .layer(SetRequestIdLayer::new(
            x_request_id.clone(),
            MakeRequestUuid,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(Level::INFO)
                        .include_headers(true),
                )
                .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
        )
        .layer(AddExtensionLayer::new(Arc::new(state)))
        .layer(RequestBodyLimitLayer::new(cfg.server.max_body))
        .layer(
            CorsLayer::new()
                .allow_origin(cfg.server.origins)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(CsrfLayer::new(csrf_secret))
        .layer(CompressionLayer::new())
        .layer(PropagateRequestIdLayer::new(x_request_id))
        .layer(DecompressionLayer::new());

    let router = axum::Router::new().route("/", get(root)).layer(middlewares);

    let http = server::Http::new(cfg.server.addr, pki, router, shutdown.subscribe())
        .with_http2()
        .await?
        .with_http3()?;

    http.join_set.join_all().await;

    Ok(())
}
