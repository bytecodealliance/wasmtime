//! Implementation of the `wasi:http/types` interface's various body types.

use crate::{bindings::http::types, types::FieldMap};
use anyhow::anyhow;
use bytes::Bytes;
use http_body::{Body, Frame};
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use std::future::Future;
use std::mem;
use std::task::{Context, Poll};
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};
use wasmtime_wasi::{
    runtime::{poll_noop, AbortOnDropJoinHandle},
    HostInputStream, HostOutputStream, StreamError, Subscribe,
};

/// Common type for incoming bodies.
pub type HyperIncomingBody = BoxBody<Bytes, types::ErrorCode>;

/// Common type for outgoing bodies.
pub type HyperOutgoingBody = BoxBody<Bytes, types::ErrorCode>;

/// The concrete type behind a `was:http/types/incoming-body` resource.
pub struct HostIncomingBody {
    body: IncomingBodyState,
    /// An optional worker task to keep alive while this body is being read.
    /// This ensures that if the parent of this body is dropped before the body
    /// then the backing data behind this worker is kept alive.
    worker: Option<AbortOnDropJoinHandle<()>>,
}

impl HostIncomingBody {
    /// Create a new `HostIncomingBody` with the given `body` and a per-frame timeout
    pub fn new(body: HyperIncomingBody, between_bytes_timeout: Duration) -> HostIncomingBody {
        let body = BodyWithTimeout::new(body, between_bytes_timeout);
        HostIncomingBody {
            body: IncomingBodyState::Start(body),
            worker: None,
        }
    }

    /// Retain a worker task that needs to be kept alive while this body is being read.
    pub fn retain_worker(&mut self, worker: AbortOnDropJoinHandle<()>) {
        assert!(self.worker.is_none());
        self.worker = Some(worker);
    }

    /// Try taking the stream of this body, if it's available.
    pub fn take_stream(&mut self) -> Option<HostIncomingBodyStream> {
        match &mut self.body {
            IncomingBodyState::Start(_) => {}
            IncomingBodyState::InBodyStream(_) => return None,
        }
        let (tx, rx) = oneshot::channel();
        let body = match mem::replace(&mut self.body, IncomingBodyState::InBodyStream(rx)) {
            IncomingBodyState::Start(b) => b,
            IncomingBodyState::InBodyStream(_) => unreachable!(),
        };
        Some(HostIncomingBodyStream {
            state: IncomingBodyStreamState::Open { body, tx },
            buffer: Bytes::new(),
            error: None,
        })
    }

    pub fn into_future_trailers(self) -> HostFutureTrailers {
        HostFutureTrailers::Waiting(self)
    }
}

/// Internal state of a [`HostIncomingBody`].
enum IncomingBodyState {
    /// The body is stored here meaning that within `HostIncomingBody` the
    /// `take_stream` method can be called for example.
    Start(BodyWithTimeout),

    /// The body is within a `HostIncomingBodyStream` meaning that it's not
    /// currently owned here. The body will be sent back over this channel when
    /// it's done, however.
    InBodyStream(oneshot::Receiver<StreamEnd>),
}

/// Small wrapper around [`HyperIncomingBody`] which adds a timeout to every frame.
struct BodyWithTimeout {
    /// Underlying stream that frames are coming from.
    inner: HyperIncomingBody,
    /// Currently active timeout that's reset between frames.
    timeout: Pin<Box<tokio::time::Sleep>>,
    /// Whether or not `timeout` needs to be reset on the next call to
    /// `poll_frame`.
    reset_sleep: bool,
    /// Maximal duration between when a frame is first requested and when it's
    /// allowed to arrive.
    between_bytes_timeout: Duration,
}

impl BodyWithTimeout {
    fn new(inner: HyperIncomingBody, between_bytes_timeout: Duration) -> BodyWithTimeout {
        BodyWithTimeout {
            inner,
            between_bytes_timeout,
            reset_sleep: true,
            timeout: Box::pin(wasmtime_wasi::runtime::with_ambient_tokio_runtime(|| {
                tokio::time::sleep(Duration::new(0, 0))
            })),
        }
    }
}

impl Body for BodyWithTimeout {
    type Data = Bytes;
    type Error = types::ErrorCode;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, types::ErrorCode>>> {
        let me = Pin::into_inner(self);

        // If the timeout timer needs to be reset, do that now relative to the
        // current instant. Otherwise test the timeout timer and see if it's
        // fired yet and if so we've timed out and return an error.
        if me.reset_sleep {
            me.timeout
                .as_mut()
                .reset(tokio::time::Instant::now() + me.between_bytes_timeout);
            me.reset_sleep = false;
        }

        // Register interest in this context on the sleep timer, and if the
        // sleep elapsed that means that we've timed out.
        if let Poll::Ready(()) = me.timeout.as_mut().poll(cx) {
            return Poll::Ready(Some(Err(types::ErrorCode::ConnectionReadTimeout)));
        }

        // Without timeout business now handled check for the frame. If a frame
        // arrives then the sleep timer will be reset on the next frame.
        let result = Pin::new(&mut me.inner).poll_frame(cx);
        me.reset_sleep = result.is_ready();
        result
    }
}

/// Message sent when a `HostIncomingBodyStream` is done to the
/// `HostFutureTrailers` state.
enum StreamEnd {
    /// The body wasn't completely read and was dropped early. May still have
    /// trailers, but requires reading more frames.
    Remaining(BodyWithTimeout),

    /// Body was completely read and trailers were read. Here are the trailers.
    /// Note that `None` means that the body finished without trailers.
    Trailers(Option<FieldMap>),
}

/// The concrete type behind the `wasi:io/streams/input-stream` resource returned
/// by `wasi:http/types/incoming-body`'s `stream` method.
pub struct HostIncomingBodyStream {
    state: IncomingBodyStreamState,
    buffer: Bytes,
    error: Option<anyhow::Error>,
}

impl HostIncomingBodyStream {
    fn record_frame(&mut self, frame: Option<Result<Frame<Bytes>, types::ErrorCode>>) {
        match frame {
            Some(Ok(frame)) => match frame.into_data() {
                // A data frame was received, so queue up the buffered data for
                // the next `read` call.
                Ok(bytes) => {
                    assert!(self.buffer.is_empty());
                    self.buffer = bytes;
                }

                // Trailers were received meaning that this was the final frame.
                // Throw away the body and send the trailers along the
                // `tx` channel to make them available.
                Err(trailers) => {
                    let trailers = trailers.into_trailers().unwrap();
                    let tx = match mem::replace(&mut self.state, IncomingBodyStreamState::Closed) {
                        IncomingBodyStreamState::Open { body: _, tx } => tx,
                        IncomingBodyStreamState::Closed => unreachable!(),
                    };

                    // NB: ignore send failures here because if this fails then
                    // no one was interested in the trailers.
                    let _ = tx.send(StreamEnd::Trailers(Some(trailers)));
                }
            },

            // An error was received meaning that the stream is now done.
            // Destroy the body to terminate the stream while enqueueing the
            // error to get returned from the next call to `read`.
            Some(Err(e)) => {
                self.error = Some(e.into());
                self.state = IncomingBodyStreamState::Closed;
            }

            // No more frames are going to be received again, so drop the `body`
            // and the `tx` channel we'd send the body back onto because it's
            // not needed as frames are done.
            None => {
                self.state = IncomingBodyStreamState::Closed;
            }
        }
    }
}

enum IncomingBodyStreamState {
    /// The body is currently open for reading and present here.
    ///
    /// When trailers are read, or when this is dropped, the body is sent along
    /// `tx`.
    ///
    /// This state is transitioned to `Closed` when an error happens, EOF
    /// happens, or when trailers are read.
    Open {
        body: BodyWithTimeout,
        tx: oneshot::Sender<StreamEnd>,
    },

    /// This body is closed and no longer available for reading, no more data
    /// will come.
    Closed,
}

#[async_trait::async_trait]
impl HostInputStream for HostIncomingBodyStream {
    fn read(&mut self, size: usize) -> Result<Bytes, StreamError> {
        loop {
            // Handle buffered data/errors if any
            if !self.buffer.is_empty() {
                let len = size.min(self.buffer.len());
                let chunk = self.buffer.split_to(len);
                return Ok(chunk);
            }

            if let Some(e) = self.error.take() {
                return Err(StreamError::LastOperationFailed(e));
            }

            // Extract the body that we're reading from. If present perform a
            // non-blocking poll to see if a frame is already here. If it is
            // then turn the loop again to operate on the results. If it's not
            // here then return an empty buffer as no data is available at this
            // time.
            let body = match &mut self.state {
                IncomingBodyStreamState::Open { body, .. } => body,
                IncomingBodyStreamState::Closed => return Err(StreamError::Closed),
            };

            let future = body.frame();
            futures::pin_mut!(future);
            match poll_noop(future) {
                Some(result) => {
                    self.record_frame(result);
                }
                None => return Ok(Bytes::new()),
            }
        }
    }
}

#[async_trait::async_trait]
impl Subscribe for HostIncomingBodyStream {
    async fn ready(&mut self) {
        if !self.buffer.is_empty() || self.error.is_some() {
            return;
        }

        if let IncomingBodyStreamState::Open { body, .. } = &mut self.state {
            let frame = body.frame().await;
            self.record_frame(frame);
        }
    }
}

impl Drop for HostIncomingBodyStream {
    fn drop(&mut self) {
        // When a body stream is dropped, for whatever reason, attempt to send
        // the body back to the `tx` which will provide the trailers if desired.
        // This isn't necessary if the state is already closed. Additionally,
        // like `record_frame` above, `send` errors are ignored as they indicate
        // that the body/trailers aren't actually needed.
        let prev = mem::replace(&mut self.state, IncomingBodyStreamState::Closed);
        if let IncomingBodyStreamState::Open { body, tx } = prev {
            let _ = tx.send(StreamEnd::Remaining(body));
        }
    }
}

/// The concrete type behind a `wasi:http/types/future-trailers` resource.
pub enum HostFutureTrailers {
    /// Trailers aren't here yet.
    ///
    /// This state represents two similar states:
    ///
    /// * The body is here and ready for reading and we're waiting to read
    ///   trailers. This can happen for example when the actual body wasn't read
    ///   or if the body was only partially read.
    ///
    /// * The body is being read by something else and we're waiting for that to
    ///   send us the trailers (or the body itself). This state will get entered
    ///   when the body stream is dropped for example. If the body stream reads
    ///   the trailers itself it will also send a message over here with the
    ///   trailers.
    Waiting(HostIncomingBody),

    /// Trailers are ready and here they are.
    ///
    /// Note that `Ok(None)` means that there were no trailers for this request
    /// while `Ok(Some(_))` means that trailers were found in the request.
    Done(Result<Option<FieldMap>, types::ErrorCode>),

    /// Trailers have been consumed by `future-trailers.get`.
    Consumed,
}

#[async_trait::async_trait]
impl Subscribe for HostFutureTrailers {
    async fn ready(&mut self) {
        let body = match self {
            HostFutureTrailers::Waiting(body) => body,
            HostFutureTrailers::Done(_) => return,
            HostFutureTrailers::Consumed => return,
        };

        // If the body is itself being read by a body stream then we need to
        // wait for that to be done.
        if let IncomingBodyState::InBodyStream(rx) = &mut body.body {
            match rx.await {
                // Trailers were read for us and here they are, so store the
                // result.
                Ok(StreamEnd::Trailers(t)) => *self = Self::Done(Ok(t)),

                // The body wasn't fully read and was dropped before trailers
                // were reached. It's up to us now to complete the body.
                Ok(StreamEnd::Remaining(b)) => body.body = IncomingBodyState::Start(b),

                // Technically this shouldn't be possible as the sender
                // shouldn't get destroyed without receiving a message. Handle
                // this just in case though.
                Err(_) => {
                    debug_assert!(false, "should be unreachable");
                    *self = HostFutureTrailers::Done(Err(types::ErrorCode::ConnectionTerminated));
                }
            }
        }

        // Here it should be guaranteed that `InBodyStream` is now gone, so if
        // we have the body ourselves then read frames until trailers are found.
        let body = match self {
            HostFutureTrailers::Waiting(body) => body,
            HostFutureTrailers::Done(_) => return,
            HostFutureTrailers::Consumed => return,
        };
        let hyper_body = match &mut body.body {
            IncomingBodyState::Start(body) => body,
            IncomingBodyState::InBodyStream(_) => unreachable!(),
        };
        let result = loop {
            match hyper_body.frame().await {
                None => break Ok(None),
                Some(Err(e)) => break Err(e),
                Some(Ok(frame)) => {
                    // If this frame is a data frame ignore it as we're only
                    // interested in trailers.
                    if let Ok(headers) = frame.into_trailers() {
                        break Ok(Some(headers));
                    }
                }
            }
        };
        *self = HostFutureTrailers::Done(result);
    }
}

#[derive(Clone)]
struct WrittenState {
    expected: u64,
    written: Arc<std::sync::atomic::AtomicU64>,
}

impl WrittenState {
    fn new(expected_size: u64) -> Self {
        Self {
            expected: expected_size,
            written: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// The number of bytes that have been written so far.
    fn written(&self) -> u64 {
        self.written.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Add `len` to the total number of bytes written. Returns `false` if the new total exceeds
    /// the number of bytes expected to be written.
    fn update(&self, len: usize) -> bool {
        let len = len as u64;
        let old = self
            .written
            .fetch_add(len, std::sync::atomic::Ordering::Relaxed);
        old + len <= self.expected
    }
}

/// The concrete type behind a `wasi:http/types/outgoing-body` resource.
pub struct HostOutgoingBody {
    pub body_output_stream: Option<Box<dyn HostOutputStream>>,
    context: StreamContext,
    written: Option<WrittenState>,
    finish_sender: Option<tokio::sync::oneshot::Sender<FinishMessage>>,
}

impl HostOutgoingBody {
    /// Create a new `HostOutgoingBody`
    pub fn new(context: StreamContext, size: Option<u64>) -> (Self, HyperOutgoingBody) {
        let written = size.map(WrittenState::new);

        use tokio::sync::oneshot::error::RecvError;
        struct BodyImpl {
            body_receiver: mpsc::Receiver<Bytes>,
            finish_receiver: Option<oneshot::Receiver<FinishMessage>>,
        }
        impl Body for BodyImpl {
            type Data = Bytes;
            type Error = types::ErrorCode;
            fn poll_frame(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
                match self.as_mut().body_receiver.poll_recv(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Some(frame)) => Poll::Ready(Some(Ok(Frame::data(frame)))),

                    // This means that the `body_sender` end of the channel has been dropped.
                    Poll::Ready(None) => {
                        if let Some(mut finish_receiver) = self.as_mut().finish_receiver.take() {
                            match Pin::new(&mut finish_receiver).poll(cx) {
                                Poll::Pending => {
                                    self.as_mut().finish_receiver = Some(finish_receiver);
                                    Poll::Pending
                                }
                                Poll::Ready(Ok(message)) => match message {
                                    FinishMessage::Finished => Poll::Ready(None),
                                    FinishMessage::Trailers(trailers) => {
                                        Poll::Ready(Some(Ok(Frame::trailers(trailers))))
                                    }
                                    FinishMessage::Abort => {
                                        Poll::Ready(Some(Err(types::ErrorCode::HttpProtocolError)))
                                    }
                                },
                                Poll::Ready(Err(RecvError { .. })) => Poll::Ready(None),
                            }
                        } else {
                            Poll::Ready(None)
                        }
                    }
                }
            }
        }

        let (body_sender, body_receiver) = mpsc::channel(2);
        let (finish_sender, finish_receiver) = oneshot::channel();
        let body_impl = BodyImpl {
            body_receiver,
            finish_receiver: Some(finish_receiver),
        }
        .boxed();

        // TODO: this capacity constant is arbitrary, and should be configurable
        let output_stream =
            BodyWriteStream::new(context, 1024 * 1024, body_sender, written.clone());

        (
            Self {
                body_output_stream: Some(Box::new(output_stream)),
                context,
                written,
                finish_sender: Some(finish_sender),
            },
            body_impl,
        )
    }

    /// Finish the body, optionally with trailers.
    pub fn finish(mut self, trailers: Option<FieldMap>) -> Result<(), types::ErrorCode> {
        // Make sure that the output stream has been dropped, so that the BodyImpl poll function
        // will immediately pick up the finish sender.
        drop(self.body_output_stream);

        let sender = self
            .finish_sender
            .take()
            .expect("outgoing-body trailer_sender consumed by a non-owning function");

        if let Some(w) = self.written {
            let written = w.written();
            if written != w.expected {
                let _ = sender.send(FinishMessage::Abort);
                return Err(self.context.as_body_error(written));
            }
        }

        let message = if let Some(ts) = trailers {
            FinishMessage::Trailers(ts)
        } else {
            FinishMessage::Finished
        };

        // Ignoring failure: receiver died sending body, but we can't report that here.
        let _ = sender.send(message.into());

        Ok(())
    }

    /// Abort the body.
    pub fn abort(mut self) {
        // Make sure that the output stream has been dropped, so that the BodyImpl poll function
        // will immediately pick up the finish sender.
        drop(self.body_output_stream);

        let sender = self
            .finish_sender
            .take()
            .expect("outgoing-body trailer_sender consumed by a non-owning function");

        let _ = sender.send(FinishMessage::Abort);
    }
}

/// Message sent to end the `[HostOutgoingBody]` stream.
enum FinishMessage {
    Finished,
    Trailers(hyper::HeaderMap),
    Abort,
}

/// Whether the body is a request or response body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamContext {
    Request,
    Response,
}

impl StreamContext {
    /// Construct an http request or response body size error.
    pub fn as_body_error(&self, size: u64) -> types::ErrorCode {
        match self {
            StreamContext::Request => types::ErrorCode::HttpRequestBodySize(Some(size)),
            StreamContext::Response => types::ErrorCode::HttpResponseBodySize(Some(size)),
        }
    }
}

/// Provides a [`HostOutputStream`] impl from a [`tokio::sync::mpsc::Sender`].
struct BodyWriteStream {
    context: StreamContext,
    writer: mpsc::Sender<Bytes>,
    write_budget: usize,
    written: Option<WrittenState>,
}

impl BodyWriteStream {
    /// Create a [`BodyWriteStream`].
    fn new(
        context: StreamContext,
        write_budget: usize,
        writer: mpsc::Sender<Bytes>,
        written: Option<WrittenState>,
    ) -> Self {
        // at least one capacity is required to send a message
        assert!(writer.max_capacity() >= 1);
        BodyWriteStream {
            context,
            writer,
            write_budget,
            written,
        }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for BodyWriteStream {
    fn write(&mut self, bytes: Bytes) -> Result<(), StreamError> {
        let len = bytes.len();
        match self.writer.try_send(bytes) {
            // If the message was sent then it's queued up now in hyper to get
            // received.
            Ok(()) => {
                if let Some(written) = self.written.as_ref() {
                    if !written.update(len) {
                        let total = written.written();
                        return Err(StreamError::LastOperationFailed(anyhow!(self
                            .context
                            .as_body_error(total))));
                    }
                }

                Ok(())
            }

            // If this channel is full then that means `check_write` wasn't
            // called. The call to `check_write` always guarantees that there's
            // at least one capacity if a write is allowed.
            Err(mpsc::error::TrySendError::Full(_)) => {
                Err(StreamError::Trap(anyhow!("write exceeded budget")))
            }

            // Hyper is gone so this stream is now closed.
            Err(mpsc::error::TrySendError::Closed(_)) => Err(StreamError::Closed),
        }
    }

    fn flush(&mut self) -> Result<(), StreamError> {
        // Flushing doesn't happen in this body stream since we're currently
        // only tracking sending bytes over to hyper.
        if self.writer.is_closed() {
            Err(StreamError::Closed)
        } else {
            Ok(())
        }
    }

    fn check_write(&mut self) -> Result<usize, StreamError> {
        if self.writer.is_closed() {
            Err(StreamError::Closed)
        } else if self.writer.capacity() == 0 {
            // If there is no more capacity in this sender channel then don't
            // allow any more writes because the hyper task needs to catch up
            // now.
            //
            // Note that this relies on this task being the only one sending
            // data to ensure that no one else can steal a write into this
            // channel.
            Ok(0)
        } else {
            Ok(self.write_budget)
        }
    }
}

#[async_trait::async_trait]
impl Subscribe for BodyWriteStream {
    async fn ready(&mut self) {
        // Attempt to perform a reservation for a send. If there's capacity in
        // the channel or it's already closed then this will return immediately.
        // If the channel is full this will block until capacity opens up.
        let _ = self.writer.reserve().await;
    }
}
