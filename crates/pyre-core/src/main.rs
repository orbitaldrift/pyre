#![allow(dead_code)]

use std::{net::SocketAddr, path::PathBuf, process::exit};

use auth::session::SessionBackend;
use axum::{response::IntoResponse, routing::get, Router};
use axum_login::{login_required, AuthSession};
use color_eyre::eyre::Context;
use config::Config;
use garde::Validate;
use pyre_build::build_info;
use pyre_cli::shutdown::Shutdown;
use pyre_crypto::tls::PkiCert;
use pyre_fs::toml::FromToml;
use pyre_telemetry::{Info, Telemetry};
use svc::{router_with_middlewares, state::AppState};
use tracing::error;
use uuid::Uuid;

mod auth;
mod config;
mod db;
mod error;
mod svc;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

async fn protect(auth_session: AuthSession<SessionBackend>) -> impl IntoResponse {
    match auth_session.user {
        Some(user) => {
            let user = user.clone();
            format!("Hello, {}!", user.name)
        }
        None => "Unauthorized".to_string(),
    }
}

async fn root() -> impl IntoResponse {
    "Hello"
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> color_eyre::Result<()> {
    // Governor axum for rate limiting per IP
    // E2E test with mock server and H3 client request, try to get auth code from discord api

    // Graphql
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

        cfg.validate().unwrap_or_else(|e| {
            error!(%e, "failed to validate config");
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

    let shutdown = Shutdown::new_with_all_signals().install();
    let pki = PkiCert::from_bytes(
        tokio::fs::read(&cfg.server.cert)
            .await
            .context("cert not found")?,
        tokio::fs::read(&cfg.server.key)
            .await
            .context("key not found")?,
    )?;

    let state = AppState::new(cfg.clone()).await?;
    let addr: SocketAddr = cfg.server.addr.parse()?;

    let router = Router::new()
        .route("/protected", get(protect))
        .route_layer(login_required!(
            SessionBackend,
            login_url = "/oauth2/discord"
        ))
        .route("/oauth2/discord", get(auth::provider::discord::redirect))
        .route("/oauth2/discord/auth", get(auth::provider::discord::auth))
        .route("/login", get(auth::login))
        .route("/", get(root));

    let router = router_with_middlewares(router, state.clone(), cfg).await?;

    let http = svc::server::Http::new(addr, pki, router, shutdown.subscribe())
        .with_http2()
        .await?
        .with_http3()?;

    http.join_set.join_all().await;
    state.shutdown().await;

    Ok(())
}
