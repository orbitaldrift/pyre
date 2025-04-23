use std::{net::SocketAddr, sync::Arc};

use axum::{body::Body, http::HeaderValue, response::Response, Router};
use futures::pin_mut;
use garde::Validate;
use hyper::{body::Incoming, Request};
use hyper_util::rt::{TokioExecutor, TokioIo};
use pyre_crypto::tls::{PkiCert, TlsServerConfig};
use pyre_transport::{stream::quinn::server::H3QuinnAcceptor, svc::axum::H3Router};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tower::Service;
use tower_http::cors::AllowOrigin;
use tracing::{error, info};

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    #[garde(range(min = 3, max = 15))]
    pub timeout: u64,
    #[garde(range(min = 1, max = 512))]
    pub max_conns: usize,
    #[garde(range(min = 4096, max = 1_000_000))] // Max is 1mb
    pub max_body: usize,
    #[garde(range(min = 1, max = 10))]
    pub limiter_period: u64,
    #[garde(range(min = 5, max = 120))]
    pub limiter_retain_interval: u64,
    #[garde(dive)]
    pub origins: Origins,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: 10,
            max_conns: 512,
            max_body: 1_000_000,
            limiter_period: 2,
            limiter_retain_interval: 60,
            origins: Origins(vec!["*".to_string()]),
        }
    }
}

#[derive(Validate, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Origins(#[garde(length(min = 1), inner(ascii, length(min = 1)))] pub Vec<String>);

impl From<Origins> for AllowOrigin {
    fn from(origins: Origins) -> Self {
        let origins = origins
            .0
            .into_iter()
            .map(|origin| {
                HeaderValue::from_str(&origin)
                    .unwrap_or_else(|_| panic!("Invalid origin header value: {origin}"))
            })
            .collect::<Vec<_>>();

        AllowOrigin::list(origins)
    }
}

pub struct Http {
    addr: SocketAddr,
    cert: PkiCert,
    router: Router,
    pub join_set: tokio::task::JoinSet<()>,
    shutdown: tokio::sync::broadcast::Receiver<()>,
}

impl Http {
    pub fn new(
        addr: SocketAddr,
        cert: PkiCert,
        router: Router,
        shutdown: tokio::sync::broadcast::Receiver<()>,
    ) -> Self {
        Self {
            addr,
            cert,
            router,
            join_set: tokio::task::JoinSet::new(),
            shutdown,
        }
    }

    pub async fn with_http2(mut self) -> color_eyre::Result<Self> {
        let tls: Arc<rustls::ServerConfig> = Arc::new(
            TlsServerConfig::new(&self.cert, vec![b"http1.1".to_vec(), b"h2".to_vec()])?.into(),
        );

        let acceptor = TlsAcceptor::from(tls.clone());
        let listener = TcpListener::bind(self.addr).await?;
        let listen_addr = listener.local_addr()?;
        let shutdown = self.shutdown.resubscribe();

        let h3_port = self.addr.port();

        let router = self.router.clone().layer(axum::middleware::map_response(
            move |mut res: Response<Body>| {
                async move {
                    res.headers_mut().insert(
                        "Alt-Svc",
                        format!("h3=\":{h3_port}\"; ma=86400").parse().unwrap(),
                    );
                    res
                }
            },
        ));

        info!(%listen_addr, "http2 listening");

        self.join_set
            .spawn(Self::http2(router, acceptor, listener, shutdown));

        Ok(self)
    }

    pub fn with_http3(mut self) -> color_eyre::Result<Self> {
        let tls: Arc<rustls::ServerConfig> =
            Arc::new(TlsServerConfig::new(&self.cert, vec![b"h3".to_vec()])?.into());

        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(tls.clone())?,
        ));
        let server = quinn::Endpoint::server(server_config, self.addr)?;
        let listen_addr = server.local_addr()?;
        let mut shutdown = self.shutdown.resubscribe();

        let acceptor = H3QuinnAcceptor::new(server);

        info!(%listen_addr, "http3 listening");

        let router = self.router.clone();
        self.join_set.spawn(async move {
            H3Router::new(router)
                .serve_with_shutdown(acceptor, async move {
                    let _ = shutdown.recv().await;
                    info!("http3 server shutting down");
                })
                .await
                .inspect_err(|e| {
                    error!(%e, "error creating h3 router");
                })
                .expect("create h3 router");
        });

        Ok(self)
    }

    async fn http2(
        router: Router,
        acceptor: TlsAcceptor,
        listener: TcpListener,
        mut shutdown: tokio::sync::broadcast::Receiver<()>,
    ) {
        async fn accept(
            router: Router,
            acceptor: TlsAcceptor,
            stream: TcpStream,
            addr: SocketAddr,
        ) {
            let Ok(stream) = acceptor.accept(stream).await else {
                error!(%addr, "error during tls handshake connection");
                return;
            };

            let mut router = router.into_make_service_with_connect_info::<SocketAddr>();
            let tower_service = router.call(addr).await.unwrap();

            let stream = TokioIo::new(stream);
            let hyper_service = hyper::service::service_fn(move |request: Request<Incoming>| {
                tower_service.clone().call(request)
            });

            let ret = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(stream, hyper_service)
                .await;

            if let Err(err) = ret {
                error!(%addr, %err, "error serving connection");
            }
        }

        pin_mut!(listener);

        loop {
            let router = router.clone();
            let acceptor = acceptor.clone();

            tokio::select! {
                _ = shutdown.recv() => {
                    info!("http2 server shutting down");
                    break;
                }
                conn = listener.accept() => {
                    match conn {
                        Ok((stream, addr)) => {
                            info!(%addr, "accepting connection");
                            tokio::spawn(accept(router, acceptor, stream, addr));
                        }
                        Err(e) => {
                            error!(%e, "error accepting connection");
                        }
                    }
                }
            }
        }
    }
}
