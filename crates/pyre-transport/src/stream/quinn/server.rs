use hyper::body::Bytes;
use tokio::task::JoinSet;

use crate::stream::server::H3Acceptor;

async fn select_conn2(
    incoming: &h3_quinn::Endpoint,
    tasks: &mut tokio::task::JoinSet<Result<h3_quinn::Connection, crate::Error>>,
) -> SelectOutputConn2 {
    tracing::debug!("select_conn");

    let incoming_stream_future = async {
        tracing::debug!("endpoint waiting accept");
        if let Some(i) = incoming.accept().await {
            tracing::debug!("endpoint accept incoming conn");
            SelectOutputConn2::NewIncoming(i)
        } else {
            tracing::debug!("endpoint accept done");
            SelectOutputConn2::Done
        }
    };
    if tasks.is_empty() {
        tracing::debug!("endpoint wait for new incoming");
        return incoming_stream_future.await;
    }
    tokio::select! {
        stream = incoming_stream_future => stream,
        accept = tasks.join_next() => {
            match accept.expect("JoinSet should never end") {
                Ok(conn) => {
                    match conn {
                        Ok(conn2) => {
                            SelectOutputConn2::NewConn(conn2)
                        },
                        Err(e) => SelectOutputConn2::ConnErr(e)
                    }
                },
                Err(e) => SelectOutputConn2::ConnErr(e.into()),
            }
        }
    }
}

enum SelectOutputConn2 {
    NewIncoming(h3_quinn::quinn::Incoming),
    NewConn(h3_quinn::Connection),
    ConnErr(crate::Error),
    Done,
}

pub struct H3QuinnAcceptor {
    ep: h3_quinn::Endpoint,
    tasks: tokio::task::JoinSet<Result<h3_quinn::Connection, crate::Error>>,
}

impl H3QuinnAcceptor {
    #[must_use]
    pub fn new(ep: h3_quinn::Endpoint) -> Self {
        Self {
            ep,
            tasks: JoinSet::default(),
        }
    }
}

impl H3Acceptor for H3QuinnAcceptor {
    type CONN = h3_quinn::Connection;
    type OS = h3_quinn::OpenStreams;
    type SS = h3_quinn::SendStream<Bytes>;
    type RS = h3_quinn::RecvStream;
    type OE = h3_quinn::ConnectionError;
    type BS = h3_quinn::BidiStream<Bytes>;

    async fn accept(&mut self) -> Result<Option<Self::CONN>, crate::Error> {
        loop {
            match select_conn2(&self.ep, &mut self.tasks).await {
                SelectOutputConn2::NewIncoming(incoming) => {
                    tracing::debug!("poll conn new incoming");
                    self.tasks.spawn(async move {
                        let conn = incoming.await.inspect_err(|e| tracing::error!("{:?}", e))?;
                        let conn = h3_quinn::Connection::new(conn);
                        tracing::debug!("incoming conn");
                        Ok(conn)
                    });
                }
                SelectOutputConn2::NewConn(connection) => {
                    return Ok(Some(connection));
                }
                SelectOutputConn2::ConnErr(error) => {
                    tracing::debug!(%error, "conn error");
                }
                SelectOutputConn2::Done => {
                    return Ok(None);
                }
            }
        }
    }
}
