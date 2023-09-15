use crate::bindings::http::types::{Headers, Method, Scheme};
use std::{pin, task};
use wasmtime_wasi::preview2::{self, AbortOnDropJoinHandle, HostInputStream, StreamState};

pub type DataReceiver = tokio::sync::mpsc::Receiver<bytes::Bytes>;

pub type HostFutureTrailers = tokio::sync::oneshot::Receiver<hyper::HeaderMap>;

pub struct HostIncomingBody {
    pub body: hyper::body::Incoming,
}

impl HostIncomingBody {
    pub fn new(body: hyper::body::Incoming) -> Self {
        Self { body }
    }

    /// Consume the state held in the [`HostIncomingBody`] to spawn a task that will drive the
    /// streaming body to completion. Data segments will be communicated out over the
    /// [`DataReceiver`] channel, and a [`HostFutureTrailers`] gives a way to block on/retrieve the
    /// trailers.
    pub fn spawn(
        mut self,
    ) -> (
        AbortOnDropJoinHandle<anyhow::Result<()>>,
        DataReceiver,
        HostFutureTrailers,
    ) {
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

        let (writer, reader) = tokio::sync::mpsc::channel(1);
        let (trailer_writer, trailer_reader) = tokio::sync::oneshot::channel();

        let handle = preview2::spawn(async move {
            while let Some(frame) = FrameFut::new(&mut self.body).await {
                // TODO: we need to actually handle errors here, right now we'll exit the loop
                // early without signaling properly to either channel that we're done.
                let frame = frame?;

                if frame.is_trailers() {
                    // We know we're not going to write any more data frames at this point, so we
                    // explicitly drop the writer so that anything waiting on the read end returns
                    // immediately.
                    drop(writer);

                    let trailers = frame.into_trailers().unwrap();

                    // TODO: this will fail in two cases:
                    // 1. we've already used the channel once, which should be imposible,
                    // 2. the read end is closed.
                    // I'm not sure how to differentiate between these two cases, or really
                    // if we need to do anything to handle either.
                    let _ = trailer_writer.send(trailers);

                    break;
                }

                assert!(frame.is_data());

                let data = frame.into_data().unwrap();

                // TODO: we need to handle send errors here. In particular, if we fail to write
                // because the reader has been dropped, we need to continue around the loop to
                // drain data frames so that we can ultimately deliver the trailers.
                let _ = writer.send(data);
            }

            Ok(())
        });

        (handle, reader, trailer_reader)
    }
}

#[async_trait::async_trait]
impl HostInputStream for HostIncomingBody {
    fn read(&mut self, _size: usize) -> anyhow::Result<(bytes::Bytes, StreamState)> {
        todo!()
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}
