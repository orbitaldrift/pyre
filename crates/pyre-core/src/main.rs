#![allow(dead_code)]

use std::{path::PathBuf, process::exit, sync::Arc};

use axum::routing::get;
use color_eyre::eyre::Context;
use config::Config;
use error::Error;
use pyre_build::build_info;
use pyre_cli::shutdown::Shutdown;
use pyre_crypto::PkiCert;
use pyre_fs::toml::FromToml;
use pyre_telemetry::{Info, Telemetry};
use svc::{add_middlewares, state::AppState};
use tracing::{error, info};
use uuid::Uuid;
mod auth;
mod config;
mod error;
mod svc;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

async fn protect() -> &'static str {
    "Protected"
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> color_eyre::Result<()> {
    // Prepare Auth Backend + Integrate with CSRF - Discord as OAuth

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

    info!(%cfg, "initializing");

    let shutdown = Shutdown::new_with_all_signals().install();
    let pki = PkiCert::from_bytes(
        tokio::fs::read(&cfg.server.cert)
            .await
            .context("cert not found")?,
        tokio::fs::read(&cfg.server.key)
            .await
            .context("key not found")?,
    )?;

    let state = Arc::new(AppState::new(cfg.db.clone()).await?);

    info!("deriving app secret");
    // TODO fetch salt from pg
    // if no salt, generate a new one and derive key with that salt
    // TODO change to secure derivation if env is prod
    let secret = pki.derive_key_fast();

    let addr = cfg.server.addr;
    let router = add_middlewares(
        state.clone(),
        axum::Router::new()
            .route("/login", get(svc::auth::login))
            .route("/protected", get(protect)),
        cfg,
        secret,
    )
    .await?;

    let http = svc::server::Http::new(addr, pki, router, shutdown.subscribe())
        .with_http2()
        .await?
        .with_http3()?;

    http.join_set.join_all().await;
    state.shutdown().await;

    Ok(())
}
