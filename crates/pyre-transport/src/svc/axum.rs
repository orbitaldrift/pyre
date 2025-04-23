use std::{future::Future, net::SocketAddr};

use axum::body::Bytes;
use hyper::{body::Body, Request, Response};

use crate::{body::server::H3IncomingServer, stream::server::H3Acceptor};

/// Accept each connection from acceptor, then for each connection
/// accept each request. Spawn a task to handle each request.
async fn serve_inner<AC, F>(
    svc: axum::Router,
    mut acceptor: AC,
    signal: F,
) -> Result<(), crate::Error>
where
    AC: H3Acceptor<ConnectInfo = SocketAddr>,
    F: Future<Output = ()>,
{
    let h_svc = hyper_util::service::TowerToHyperService::new(svc);

    let mut sig = std::pin::pin!(signal);
    tracing::debug!("loop start");

    loop {
        tracing::debug!("loop");

        let result = tokio::select! {
            res = acceptor.accept() => {
                match res {
                    Ok(x) => x,
                    Err(e) => {
                        tracing::error!("accept error : {e}");
                        return Err(e);
                    }
                }
            }
            () = &mut sig =>{
                tracing::debug!("cancellation triggered");
                return Ok(());
            }
        };

        // Get connection and connection info
        let Some((conn, addr)) = result else {
            tracing::debug!("acceptor end of conn");
            return Ok(());
        };

        let h_svc_cp = h_svc.clone();
        tokio::spawn(async move {
            let mut conn = match h3::server::Connection::new(conn).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("server connection failed: {}", e);
                    return;
                }
            };
            loop {
                let (request, stream) = match conn.accept().await {
                    Ok(req) => {
                        if let Some(r) = req {
                            r
                        } else {
                            tracing::debug!("server connection ended:");
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("server connection accept failed: {}", e);
                        break;
                    }
                };
                let h_svc_cp = h_svc_cp.clone();
                let addr_clone = addr;
                tokio::spawn(async move {
                    if let Err(e) = serve_request_with_addr::<AC, _, _>(
                        request,
                        stream,
                        h_svc_cp.clone(),
                        addr_clone,
                    )
                    .await
                    {
                        tracing::error!("server request failed: {}", e);
                    }
                });
            }
        });
    }
}

async fn serve_request_with_addr<AC, SVC, BD>(
    request: Request<()>,
    stream: h3::server::RequestStream<
        <<AC as H3Acceptor>::CONN as h3::quic::OpenStreams<Bytes>>::BidiStream,
        Bytes,
    >,
    service: SVC,
    addr: SocketAddr,
) -> Result<(), crate::Error>
where
    AC: H3Acceptor<ConnectInfo = SocketAddr>,
    SVC: hyper::service::Service<
        Request<H3IncomingServer<AC::RS, Bytes>>,
        Response = Response<BD>,
        Error = std::convert::Infallible,
    >,
    SVC::Future: 'static,
    BD: Body + 'static,
    BD::Error: Into<crate::Error>,
    <BD as Body>::Error: Into<crate::Error> + std::error::Error + Send + Sync,
    <BD as Body>::Data: Send + Sync,
{
    let (mut parts, ()) = request.into_parts();
    let (mut w, r) = stream.split();

    // Add the connection info as an extension
    parts.extensions.insert(axum::extract::ConnectInfo(addr));

    let request = Request::from_parts(parts, H3IncomingServer::new(r));

    let response = service.call(request).await?;
    let (res_h, res_b) = response.into_parts();

    w.send_response(Response::from_parts(res_h, ())).await?;

    crate::body::server::send_h3_server_body::<BD, AC::BS>(&mut w, res_b).await?;

    Ok(())
}

pub struct H3Router(axum::Router);

impl H3Router {
    #[must_use]
    pub fn new(inner: axum::Router) -> Self {
        Self(inner)
    }
}

impl H3Router {
    /// Runs the service on acceptor until shutdown.
    ///
    /// # Errors
    /// If the acceptor fails to accept a connection.
    pub async fn serve_with_shutdown<AC, F>(
        self,
        acceptor: AC,
        signal: F,
    ) -> Result<(), crate::Error>
    where
        AC: H3Acceptor<ConnectInfo = SocketAddr>,
        F: Future<Output = ()>,
    {
        serve_inner(self.0, acceptor, signal).await
    }
}
