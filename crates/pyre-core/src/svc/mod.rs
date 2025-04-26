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

pub async fn start(
    cfg: Config,
    shutdown: tokio::sync::broadcast::Receiver<()>,
) -> color_eyre::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

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

    let http = Http::new(addr, pki, router, shutdown)
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
    use std::sync::Arc;

    use axum::http;
    use h3::client;
    use pyre_cli::shutdown::Shutdown;
    use pyre_telemetry::Telemetry;
    use quinn::crypto::rustls::QuicClientConfig;

    use crate::svc;

    #[tokio::test]
    async fn test_http2() {
        let _t = Telemetry::default().init_scoped();
        let shutdown = Shutdown::new_with_all_signals().install();

        let cfg = super::Config::default();
        tokio::spawn(svc::start(cfg, shutdown.subscribe()));
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // TODO: Discord

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .http2_prior_knowledge()
            .build()
            .unwrap();
        let res = client.get("https://localhost:4433/").send().await.unwrap();
        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn test_http3() {
        let _t = Telemetry::default().init_scoped();
        let shutdown = Shutdown::new_with_all_signals().install();

        let cfg = super::Config::default();
        tokio::spawn(svc::start(cfg, shutdown.subscribe()));
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let provider = rustls::crypto::ring::default_provider()
            .install_default()
            .err()
            .unwrap();

        let mut client_crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(danger::NoCertificateVerification {}))
            .with_no_client_auth();
        client_crypto.alpn_protocols = vec![b"h3".to_vec()];

        let suite = provider
            .cipher_suites
            .iter()
            .find_map(|cs| {
                match (cs.suite(), cs.tls13()) {
                    (rustls::CipherSuite::TLS13_AES_128_GCM_SHA256, Some(suite)) => {
                        Some(suite.quic_suite())
                    }
                    _ => None,
                }
            })
            .flatten()
            .unwrap();
        let quic_client_config =
            QuicClientConfig::with_initial(Arc::new(client_crypto), suite).unwrap();
        let quinn_proto_config = quinn_proto::ClientConfig::new(Arc::new(quic_client_config));

        let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().unwrap()).unwrap();
        endpoint.set_default_client_config(quinn_proto_config);

        let connection = endpoint
            .connect("127.0.0.1:4433".parse().unwrap(), "localhost")
            .unwrap()
            .await
            .unwrap();

        let mut h3_connection = client::builder()
            .build(h3_quinn::Connection::new(connection))
            .await
            .unwrap();

        let request = http::Request::builder()
            .uri("https://localhost:4433/")
            .method("GET")
            .body(())
            .unwrap();

        let mut response: client::RequestStream<h3_quinn::BidiStream<&[u8]>, _> =
            h3_connection.1.send_request(request).await.unwrap();
        let res = response.recv_response().await.unwrap();

        assert_eq!(res.status(), 200);
    }

    mod danger {
        use rustls::{
            client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
            SignatureScheme,
        };

        #[derive(Debug)]
        pub struct NoCertificateVerification {}

        impl ServerCertVerifier for NoCertificateVerification {
            fn verify_server_cert(
                &self,
                _end_entity: &rustls::pki_types::CertificateDer<'_>,
                _intermediates: &[rustls::pki_types::CertificateDer<'_>],
                _server_name: &rustls::pki_types::ServerName<'_>,
                _ocsp_response: &[u8],
                _now: rustls::pki_types::UnixTime,
            ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
                Ok(ServerCertVerified::assertion())
            }

            fn verify_tls12_signature(
                &self,
                _message: &[u8],
                _cert: &rustls::pki_types::CertificateDer<'_>,
                _dss: &rustls::DigitallySignedStruct,
            ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error>
            {
                Ok(HandshakeSignatureValid::assertion())
            }

            fn verify_tls13_signature(
                &self,
                _message: &[u8],
                _cert: &rustls::pki_types::CertificateDer<'_>,
                _dss: &rustls::DigitallySignedStruct,
            ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error>
            {
                Ok(HandshakeSignatureValid::assertion())
            }

            fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
                vec![SignatureScheme::ECDSA_NISTP256_SHA256]
            }
        }
    }
}
