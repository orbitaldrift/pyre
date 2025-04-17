use h3::server::RequestStream;
use hyper::body::{Body, Buf, Bytes};
use tracing::error;

pub struct H3IncomingServer<S, B>
where
    B: Buf,
    S: h3::quic::RecvStream,
{
    s: RequestStream<S, B>,
    data_done: bool,
    trailers_received: bool,
}

impl<S, B> H3IncomingServer<S, B>
where
    B: Buf,
    S: h3::quic::RecvStream,
{
    pub fn new(s: RequestStream<S, B>) -> Self {
        Self {
            s,
            data_done: false,
            trailers_received: false,
        }
    }
}

impl<S, B> hyper::body::Body for H3IncomingServer<S, B>
where
    B: Buf,
    S: h3::quic::RecvStream,
{
    type Data = hyper::body::Bytes;

    type Error = h3::Error;

    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        if self.data_done {
            let p = if let Some(tr) = futures::ready!(self.s.poll_recv_trailers(cx))? {
                self.trailers_received = true;
                std::task::Poll::Ready(Some(Ok(hyper::body::Frame::trailers(tr))))
            } else {
                self.trailers_received = true;
                std::task::Poll::Ready(None)
            };
            return p;
        }

        match futures::ready!(self.s.poll_recv_data(cx)) {
            Ok(data_opt) => {
                if let Some(mut data) = data_opt {
                    std::task::Poll::Ready(Some(Ok(hyper::body::Frame::data(
                        data.copy_to_bytes(data.remaining()),
                    ))))
                } else {
                    self.data_done = true;
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
            }
            Err(e) => std::task::Poll::Ready(Some(Err(e))),
        }
    }

    fn is_end_stream(&self) -> bool {
        self.data_done && self.trailers_received
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        hyper::body::SizeHint::default()
    }
}

/// Sends the body of a response.
///
/// # Panics
/// If frame matches data but cannot parse.
///
/// # Errors
/// If the stream cannot be sent or if the body cannot be sent.
pub async fn send_h3_server_body<BD, S>(
    w: &mut h3::server::RequestStream<<S as h3::quic::BidiStream<Bytes>>::SendStream, Bytes>,
    bd: BD,
) -> Result<(), crate::Error>
where
    BD: Body + 'static,
    BD::Error: Into<crate::Error>,
    <BD as Body>::Error: Into<crate::Error> + std::error::Error + Send + Sync,
    <BD as Body>::Data: Send + Sync,
    S: h3::quic::BidiStream<hyper::body::Bytes>,
{
    let mut p_b = std::pin::pin!(bd);
    while let Some(d) = futures::future::poll_fn(|cx| p_b.as_mut().poll_frame(cx)).await {
        let d = d.map_err(crate::Error::from)?;

        if d.is_data() {
            let mut d = d
                .into_data()
                .inspect_err(|_| {
                    error!("expected data frame");
                })
                .ok()
                .unwrap();

            // Bytes optimizes the shallow copy.
            w.send_data(d.copy_to_bytes(d.remaining())).await?;
        } else if d.is_trailers() {
            let d = d.into_trailers().ok().unwrap();

            w.send_trailers(d).await?;
        }
    }

    // Close the stream gracefully.
    // This is technically only needed when not writing trailers.
    // But msquic-h3 requires stream be gracefully closed all the time.
    w.finish().await?;
    Ok(())
}
