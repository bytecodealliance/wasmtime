use crate::bindings::http::types::{Headers, Method, Scheme};
use bytes::Bytes;
use std::{pin, task};
use tokio::sync::{mpsc, oneshot};
use wasmtime_wasi::preview2::{self, AbortOnDropJoinHandle, HostInputStream, StreamState};

pub struct HostIncomingBody {
    pub worker: AbortOnDropJoinHandle<()>,
    pub body: Option<HostIncomingBodyStream>,
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
    fn read(&mut self, size: usize) -> anyhow::Result<(bytes::Bytes, StreamState)> {
        if !self.buffer.is_empty() {
            let len = size.min(self.buffer.len());
        }
        todo!()
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}

pub struct HostFutureTrailers {
    pub worker: AbortOnDropJoinHandle<()>,
    pub received: Option<hyper::HeaderMap>,
    pub receiver: oneshot::Receiver<Result<hyper::HeaderMap, hyper::Error>>,
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
            body: Some(HostIncomingBodyStream::new(body_receiver)),
            trailers,
        }
    }
}
