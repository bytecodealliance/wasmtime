use crate::{
    bindings::http::types,
    types::FieldMap,
};
use bytes::Bytes;
use std::{pin, task};
use tokio::sync::{mpsc, oneshot};
use wasmtime_wasi::preview2::{self, AbortOnDropJoinHandle, HostInputStream, StreamState};

pub struct HostIncomingBody {
    pub worker: AbortOnDropJoinHandle<()>,
    pub stream: Option<HostIncomingBodyStream>,
    pub trailers: oneshot::Receiver<Result<hyper::HeaderMap, hyper::Error>>,
}

impl HostIncomingBody {
    pub fn into_future_trailers(self) -> HostFutureTrailers {
        HostFutureTrailers {
            worker: self.worker,
            receiver: self.trailers,
            received: None,
        }
    }
}

pub struct HostIncomingBodyStream {
    pub open: bool,
    pub receiver: mpsc::Receiver<Result<Bytes, hyper::Error>>,
    pub buffer: Bytes,
    pub error: Option<hyper::Error>,
}

impl HostIncomingBodyStream {
    fn new(receiver: mpsc::Receiver<Result<Bytes, hyper::Error>>) -> Self {
        Self {
            open: true,
            receiver,
            buffer: Bytes::new(),
            error: None,
        }
    }
}

#[async_trait::async_trait]
impl HostInputStream for HostIncomingBodyStream {
    fn read(&mut self, size: usize) -> anyhow::Result<(Bytes, StreamState)> {
        use mpsc::error::TryRecvError;

        if !self.buffer.is_empty() {
            let len = size.min(self.buffer.len());
            let chunk = self.buffer.split_to(len);
            return Ok((chunk, StreamState::Open));
        }

        // TODO: we need to check self.error and report it, once we have the means to do so through
        // the streams interface.

        if !self.open {
            return Ok((Bytes::new(), StreamState::Closed));
        }

        match self.receiver.try_recv() {
            Ok(Ok(mut bytes)) => {
                let len = bytes.len().min(size);
                let chunk = bytes.split_to(len);
                if !bytes.is_empty() {
                    self.buffer = bytes;
                }

                return Ok((chunk, StreamState::Open));
            }

            Ok(Err(e)) => {
                self.open = false;
                self.error = Some(e);
                return Ok((Bytes::new(), StreamState::Closed));
            }

            Err(TryRecvError::Empty) => {
                return Ok((Bytes::new(), StreamState::Open));
            }

            Err(TryRecvError::Disconnected) => {
                self.open = false;
                return Ok((Bytes::new(), StreamState::Closed));
            }
        }
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        if !self.buffer.is_empty() {
            return Ok(());
        }

        if !self.open {
            return Ok(());
        }

        match self.receiver.recv().await {
            Some(Ok(bytes)) => self.buffer = bytes,

            Some(Err(e)) => {
                self.error = Some(e);
                self.open = false;
            }

            None => self.open = false,
        }

        Ok(())
    }
}

impl HostIncomingBody {
    /// Consume the state held in the [`HostIncomingBody`] to spawn a task that will drive the
    /// streaming body to completion. Data segments will be communicated out over the
    /// [`DataReceiver`] channel, and a [`HostFutureTrailers`] gives a way to block on/retrieve the
    /// trailers.
    pub fn new(mut body: hyper::body::Incoming) -> Self {
        use hyper::body::{Body, Frame};

        struct FrameFut<'a> {
            body: &'a mut hyper::body::Incoming,
        }

        impl<'a> FrameFut<'a> {
            fn new(body: &'a mut hyper::body::Incoming) -> Self {
                Self { body }
            }
        }

        impl<'a> std::future::Future for FrameFut<'a> {
            type Output = Option<Result<Frame<bytes::Bytes>, hyper::Error>>;

            fn poll(
                mut self: pin::Pin<&mut Self>,
                cx: &mut task::Context<'_>,
            ) -> task::Poll<Self::Output> {
                if self.body.is_end_stream() {
                    return task::Poll::Ready(None);
                }

                pin::Pin::new(&mut self.body).poll_frame(cx)
            }
        }

        let (body_writer, body_receiver) = mpsc::channel(1);
        let (trailer_writer, trailers) = oneshot::channel();

        let worker = preview2::spawn(async move {
            while let Some(frame) = FrameFut::new(&mut body).await {
                // TODO: we need to actually handle errors here, right now we'll exit the loop
                // early without signaling properly to either channel that we're done.
                if let Err(e) = frame {
                    match body_writer.send(Err(e)).await {
                        Ok(_) => {}
                        // If the body read end has dropped, then we report this error with the
                        // trailers. unwrap and rewrap Err because the Ok side of these two Results
                        // are different.
                        Err(e) => {
                            let _ = trailer_writer.send(Err(e.0.unwrap_err()));
                        }
                    }
                    break;
                }
                let frame = frame.unwrap();

                if frame.is_trailers() {
                    // We know we're not going to write any more data frames at this point, so we
                    // explicitly drop the body_writer so that anything waiting on the read end returns
                    // immediately.
                    drop(body_writer);

                    let trailers = frame.into_trailers().unwrap();

                    // TODO: this will fail in two cases:
                    // 1. we've already used the channel once, which should be imposible,
                    // 2. the read end is closed.
                    // I'm not sure how to differentiate between these two cases, or really
                    // if we need to do anything to handle either.
                    let _ = trailer_writer.send(Ok(trailers));

                    break;
                }

                assert!(frame.is_data());

                let data = frame.into_data().unwrap();

                // If the receiver no longer exists, thats ok - in that case we want to keep the
                // loop running to relieve backpressure, so we get to the trailers.
                let _ = body_writer.send(Ok(data));
            }
        });

        Self {
            worker,
            stream: Some(HostIncomingBodyStream::new(body_receiver)),
            trailers,
        }
    }
}

pub struct HostFutureTrailers {
    pub worker: AbortOnDropJoinHandle<()>,
    pub received: Option<Result<FieldMap, types::Error>>,
    pub receiver: oneshot::Receiver<Result<hyper::HeaderMap, hyper::Error>>,
}

impl HostFutureTrailers {
    pub fn ready(&mut self) -> impl std::future::Future<Output = anyhow::Result<()>> + '_ {
        use std::future::Future;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        struct TrailersReady<'a>(&'a mut HostFutureTrailers);

        impl<'a> Future for TrailersReady<'a> {
            type Output = anyhow::Result<()>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.0.received.is_some() {
                    return Poll::Ready(Ok(()));
                }

                match Pin::new(&mut self.0.receiver).poll(cx) {
                    Poll::Ready(Ok(Ok(headers))) => {
                        self.0.received = Some(Ok(FieldMap::from(headers)))
                    }

                    Poll::Ready(Ok(Err(e))) => {
                        self.0.received = Some(Err(types::Error::ProtocolError(format!(
                            "hyper error: {e:?}"
                        ))))
                    }

                    Poll::Ready(Err(_)) => {
                        self.0.received = Some(Err(types::Error::ProtocolError(
                            "stream hung up before trailers were received".to_string(),
                        )))
                    }

                    Poll::Pending => return Poll::Pending,
                }

                Poll::Ready(Ok(()))
            }
        }

        TrailersReady(self)
    }
}
