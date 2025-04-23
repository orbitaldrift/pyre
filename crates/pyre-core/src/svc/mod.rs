use std::net::SocketAddr;

use axum::{
    extract::ConnectInfo,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use axum_login::login_required;
use color_eyre::eyre::Context;
use garde::Validate;
use limiter::UserIdKeyExtractor;
use middleware::router_with_middlewares;
use pyre_cli::shutdown::Shutdown;
use pyre_crypto::tls::PkiCert;
use serde::{Deserialize, Serialize};
use server::Http;
use state::AppState;
use tower_governor::GovernorLayer;
use tracing::info;

use crate::{
    auth::{self, session::SessionBackend},
    config::Config,
};

pub mod limiter;
pub mod middleware;
pub mod server;
pub mod state;

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

pub async fn start(cfg: Config) -> color_eyre::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let shutdown = Shutdown::new_with_all_signals().install();
    let pki = if cfg.server.test_cert {
        PkiCert::new_self_signed()?
    } else {
        PkiCert::from_bytes(
            tokio::fs::read(&cfg.server.cert)
                .await
                .context("cert not found")?,
            tokio::fs::read(&cfg.server.key)
                .await
                .context("key not found")?,
        )?
    };

    let state = AppState::new(cfg.clone()).await?;
    let addr: SocketAddr = cfg.server.addr.parse()?;

    let router = Router::new()
        .route("/me", post(auth::me))
        .route_layer(login_required!(
            SessionBackend,
            login_url = "/oauth2/discord"
        ))
        .route("/oauth2/discord", get(auth::provider::discord::redirect))
        .route("/oauth2/discord/auth", get(auth::provider::discord::auth))
        .route("/", get(root))
        .layer(GovernorLayer {
            config: limiter::setup(&cfg.http, UserIdKeyExtractor::<SessionBackend>::new()),
        });

    let router = router_with_middlewares(router, state.clone(), cfg).await?;

    let http = Http::new(addr, pki, router, shutdown.subscribe())
        .with_http2()
        .await?
        .with_http3()?;

    http.join_set.join_all().await;
    state.shutdown().await;

    Ok(())
}

async fn root(connect_info: ConnectInfo<SocketAddr>) -> impl IntoResponse {
    info!("conn info: {}", connect_info.0);

    "Hello"
}

#[cfg(test)]
mod tests {
    use pyre_telemetry::Telemetry;

    use crate::svc;

    #[tokio::test]
    async fn test_start() {
        Telemetry::default().init().unwrap();

        let cfg = super::Config::default();
        tokio::spawn(svc::start(cfg));
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // TODO: Discord + HTTP3

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .http2_prior_knowledge()
            .build()
            .unwrap();
        let res = client.get("https://localhost:4433/").send().await.unwrap();
        assert_eq!(res.status(), 200);
    }
}
