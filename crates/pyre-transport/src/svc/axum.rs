use std::future::Future;

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
    AC: H3Acceptor,
    F: Future<Output = ()>,
{
    // TODO: tonic body is wrapped? Is it for error to status conversion?
    // use tower::ServiceExt;
    // let h_svc =
    //     hyper_util::service::TowerToHyperService::new(svc.map_request(|req: http::Request<_>| {
    //         req.map(tonic::body::boxed::<crate::H3IncomingServer<AC::RS, Bytes>>)
    //     }));

    let h_svc = hyper_util::service::TowerToHyperService::new(svc);

    let mut sig = std::pin::pin!(signal);
    tracing::debug!("loop start");

    loop {
        tracing::debug!("loop");

        let conn = tokio::select! {
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

        let Some(conn) = conn else {
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
                tokio::spawn(async move {
                    if let Err(e) =
                        serve_request::<AC, _, _>(request, stream, h_svc_cp.clone()).await
                    {
                        tracing::error!("server request failed: {}", e);
                    }
                });
            }
        });
    }
}

async fn serve_request<AC, SVC, BD>(
    request: Request<()>,
    stream: h3::server::RequestStream<
        <<AC as H3Acceptor>::CONN as h3::quic::OpenStreams<Bytes>>::BidiStream,
        Bytes,
    >,
    service: SVC,
) -> Result<(), crate::Error>
where
    AC: H3Acceptor,
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
    let (parts, ()) = request.into_parts();
    let (mut w, r) = stream.split();

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

impl From<axum::Router> for H3Router {
    fn from(value: axum::Router) -> Self {
        Self::new(value)
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
        AC: H3Acceptor,
        F: Future<Output = ()>,
    {
        serve_inner(self.0, acceptor, signal).await
    }

    /// Runs all services on acceptor
    ///
    /// # Errors
    /// If the acceptor fails to accept a connection.
    pub async fn serve<AC>(self, acceptor: AC) -> Result<(), crate::Error>
    where
        AC: H3Acceptor,
    {
        self.serve_with_shutdown(acceptor, async { futures::future::pending::<()>().await })
            .await
    }
}
