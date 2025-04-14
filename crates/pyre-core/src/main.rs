use std::{net::SocketAddr, path::PathBuf, process::exit, sync::Arc};

use axum::{extract::Request, http::Response, routing::get, Router};
use color_eyre::eyre::Context;
use config::Config;
use futures::pin_mut;
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use pyre_build::build_info;
use pyre_cli::shutdown::Shutdown;
use pyre_crypto::{PkiCert, TlsServerConfig};
use pyre_fs::toml::FromToml;
use pyre_telemetry::{Info, Telemetry};
use pyre_transport::{stream::quinn::server::H3QuinnAcceptor, svc::axum::H3Router};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_rustls::TlsAcceptor;
use tower_service::Service;
use tracing::{error, info};
use uuid::Uuid;

mod config;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

async fn root() -> &'static str {
    "Hello, World from axum!"
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> color_eyre::Result<()> {
    // Metrics middleware
    // Span context propagator
    // Security headers layer
    // CORS layer
    // Rate limit layer
    // Session layer
    // zstd Compression layer
    // Timeout layer
    // Request ID layer

    build_info!();
    color_eyre::install()?;
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cfg;
    {
        let _g = Telemetry::stdout();

        (cfg, _) = Config::from_toml_path::<PathBuf>(None)
            .await
            .unwrap_or_else(|e| {
                error!("failed to load config: {e}");
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
            error!("failed to initialize telemetry: {e}");
            exit(1)
        })
        .expect("failed to initialize telemetry");
    }

    let shutdown = Shutdown::new_with_all_signals().install();

    let addr: SocketAddr = cfg.server.addr.parse()?;

    let cert = CertificateDer::from(
        tokio::fs::read(cfg.server.cert.clone())
            .await
            .context("cert not found")?,
    );
    let key = PrivateKeyDer::try_from(
        tokio::fs::read(cfg.server.key.clone())
            .await
            .context("key not found")?,
    )
    .expect("failed to load private key");
    let pki = PkiCert { cert, key };

    let h2 = start_h2(
        addr,
        Arc::new(
            TlsServerConfig::new(
                &pki,
                vec![b"http1.1".to_vec(), b"h2".to_vec(), b"h3".to_vec()],
            )?
            .into(),
        ),
        axum::Router::new()
            .route("/", get(root))
            .layer(axum::middleware::map_response(move |res| {
                set_header(res, addr.port())
            })),
        shutdown.subscribe(),
    )
    .await?;

    let h3 = start_h3(
        addr,
        Arc::new(TlsServerConfig::new(&pki, vec![b"h3".to_vec()])?.into()),
        axum::Router::new().route("/", get(root)),
        shutdown.subscribe(),
    )
    .await?;

    tokio::select! {
        _ = h3 => {
            info!("http3 server exited");
        }
        _ = h2 => {
            info!("http2 server exited");
        }
    }
    Ok(())
}

async fn start_h2(
    addr: SocketAddr,
    tls: Arc<rustls::ServerConfig>,
    router: Router,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
) -> color_eyre::Result<JoinHandle<()>> {
    let tls_acceptor = TlsAcceptor::from(tls);
    let tcp_listener = TcpListener::bind(addr).await?;

    info!("http2 listening on {}", tcp_listener.local_addr()?);

    Ok(tokio::spawn(async move {
        pin_mut!(tcp_listener);

        loop {
            let router = router.clone();
            let tls_acceptor = tls_acceptor.clone();

            tokio::select! {
                _ = shutdown.recv() => {
                    info!("http2 server shutting down");
                    break;
                }
                acc = tcp_listener.accept() => {
                    match acc {
                        Ok((cnx, addr)) => {
                            info!("accepting connection from {}", addr);

                            tokio::spawn(async move {
                                let Ok(stream) = tls_acceptor.accept(cnx).await else {
                                    error!("error during tls handshake connection from {}", addr);
                                    return;
                                };

                                let stream = TokioIo::new(stream);

                                let hyper_service =
                                    hyper::service::service_fn(move |request: Request<Incoming>| {
                                        router.clone().call(request)
                                    });

                                let ret = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                                    .serve_connection_with_upgrades(stream, hyper_service)
                                    .await;

                                if let Err(err) = ret {
                                    error!("error serving connection from {}: {}", addr, err);
                                }
                            });
                        }
                        Err(e) => {
                            error!("error accepting connection: {}", e);
                            continue;
                        }
                    }
                }
            }
        }
    }))
}

async fn start_h3(
    addr: SocketAddr,
    tls: Arc<rustls::ServerConfig>,
    router: Router,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
) -> color_eyre::Result<JoinHandle<()>> {
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls.clone()).unwrap(),
    ));
    let server = quinn::Endpoint::server(server_config, addr).unwrap();
    let listen_addr = server.local_addr().unwrap();

    let acceptor = H3QuinnAcceptor::new(server);

    info!("http3 listening on {}", listen_addr);

    Ok(tokio::spawn(async move {
        H3Router::new(router)
            .serve_with_shutdown(acceptor, async move {
                let _ = shutdown.recv().await;
                info!("http3 server shutting down");
            })
            .await
            .unwrap();
    }))
}

async fn set_header<B>(mut response: Response<B>, port: u16) -> Response<B> {
    response.headers_mut().insert(
        "Alt-Svc",
        format!("h3=\":{port}\"; ma=86400").parse().unwrap(),
    );
    response
}
