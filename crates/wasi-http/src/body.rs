use crate::{bindings::http::types, types::FieldMap};
use anyhow::anyhow;
use bytes::Bytes;
use http_body::{Body, Frame};
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use std::future::Future;
use std::mem;
use std::task::{Context, Poll};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use wasmtime_wasi::preview2::{
    self, poll_noop, AbortOnDropJoinHandle, HostInputStream, HostOutputStream, StreamError,
    Subscribe,
};

pub type HyperIncomingBody = BoxBody<Bytes, types::Error>;

/// Small wrapper around `BoxBody` which adds a timeout to every frame.
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
            timeout: Box::pin(preview2::with_ambient_tokio_runtime(|| {
                tokio::time::sleep(Duration::new(0, 0))
            })),
        }
    }
}

impl Body for BodyWithTimeout {
    type Data = Bytes;
    type Error = types::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, types::Error>>> {
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
            return Poll::Ready(Some(Err(types::Error::TimeoutError(
                "frame timed out".to_string(),
            ))));
        }

        // Without timeout business now handled check for the frame. If a frame
        // arrives then the sleep timer will be reset on the next frame.
        let result = Pin::new(&mut me.inner).poll_frame(cx);
        me.reset_sleep = result.is_ready();
        result
    }
}

pub struct HostIncomingBody {
    body: IncomingBodyState,
    /// An optional worker task to keep alive while this body is being read.
    /// This ensures that if the parent of this body is dropped before the body
    /// then the backing data behind this worker is kept alive.
    worker: Option<Arc<AbortOnDropJoinHandle<()>>>,
}

enum IncomingBodyState {
    /// The body is stored here meaning that within `HostIncomingBody` the
    /// `take_stream` method can be called for example.
    Start(BodyWithTimeout),

    /// The body is within a `HostIncomingBodyStream` meaning that it's not
    /// currently owned here. The body will be sent back over this channel when
    /// it's done, however.
    InBodyStream(oneshot::Receiver<StreamEnd>),
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

impl HostIncomingBody {
    pub fn new(body: HyperIncomingBody, between_bytes_timeout: Duration) -> HostIncomingBody {
        let body = BodyWithTimeout::new(body, between_bytes_timeout);
        HostIncomingBody {
            body: IncomingBodyState::Start(body),
            worker: None,
        }
    }

    pub fn retain_worker(&mut self, worker: &Arc<AbortOnDropJoinHandle<()>>) {
        assert!(self.worker.is_none());
        self.worker = Some(worker.clone());
    }

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

pub struct HostIncomingBodyStream {
    state: IncomingBodyStreamState,
    buffer: Bytes,
    error: Option<anyhow::Error>,
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

impl HostIncomingBodyStream {
    fn record_frame(&mut self, frame: Option<Result<Frame<Bytes>, types::Error>>) {
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
    Done(Result<Option<FieldMap>, types::Error>),
}

#[async_trait::async_trait]
impl Subscribe for HostFutureTrailers {
    async fn ready(&mut self) {
        let body = match self {
            HostFutureTrailers::Waiting(body) => body,
            HostFutureTrailers::Done(_) => return,
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
                    *self = HostFutureTrailers::Done(Err(types::Error::ProtocolError(
                        "stream hung up before trailers were received".to_string(),
                    )));
                }
            }
        }

        // Here it should be guaranteed that `InBodyStream` is now gone, so if
        // we have the body ourselves then read frames until trailers are found.
        let body = match self {
            HostFutureTrailers::Waiting(body) => body,
            HostFutureTrailers::Done(_) => return,
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

pub type HyperOutgoingBody = BoxBody<Bytes, types::Error>;

pub enum FinishMessage {
    Finished,
    Trailers(hyper::HeaderMap),
    Abort,
}

pub struct HostOutgoingBody {
    pub body_output_stream: Option<Box<dyn HostOutputStream>>,
    pub finish_sender: Option<tokio::sync::oneshot::Sender<FinishMessage>>,
}

impl HostOutgoingBody {
    pub fn new() -> (Self, HyperOutgoingBody) {
        use tokio::sync::oneshot::error::RecvError;
        struct BodyImpl {
            body_receiver: mpsc::Receiver<Bytes>,
            finish_receiver: Option<oneshot::Receiver<FinishMessage>>,
        }
        impl Body for BodyImpl {
            type Data = Bytes;
            type Error = types::Error;
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
                                    FinishMessage::Abort => Poll::Ready(Some(Err(
                                        types::Error::ProtocolError("response corrupted".into()),
                                    ))),
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

        let (body_sender, body_receiver) = mpsc::channel(1);
        let (finish_sender, finish_receiver) = oneshot::channel();
        let body_impl = BodyImpl {
            body_receiver,
            finish_receiver: Some(finish_receiver),
        }
        .boxed();
        (
            Self {
                // TODO: this capacity constant is arbitrary, and should be configurable
                body_output_stream: Some(Box::new(BodyWriteStream::new(1024 * 1024, body_sender))),
                finish_sender: Some(finish_sender),
            },
            body_impl,
        )
    }
}

// copied in from preview2::write_stream

#[derive(Debug)]
struct WorkerState {
    alive: bool,
    items: std::collections::VecDeque<Bytes>,
    write_budget: usize,
    flush_pending: bool,
    error: Option<anyhow::Error>,
}

impl WorkerState {
    fn check_error(&mut self) -> Result<(), StreamError> {
        if let Some(e) = self.error.take() {
            return Err(StreamError::LastOperationFailed(e));
        }
        if !self.alive {
            return Err(StreamError::Closed);
        }
        Ok(())
    }
}

struct Worker {
    state: Mutex<WorkerState>,
    new_work: tokio::sync::Notify,
    write_ready_changed: tokio::sync::Notify,
}

enum Job {
    Flush,
    Write(Bytes),
}

impl Worker {
    fn new(write_budget: usize) -> Self {
        Self {
            state: Mutex::new(WorkerState {
                alive: true,
                items: std::collections::VecDeque::new(),
                write_budget,
                flush_pending: false,
                error: None,
            }),
            new_work: tokio::sync::Notify::new(),
            write_ready_changed: tokio::sync::Notify::new(),
        }
    }
    async fn ready(&self) {
        loop {
            {
                let state = self.state();
                if state.error.is_some()
                    || !state.alive
                    || (!state.flush_pending && state.write_budget > 0)
                {
                    return;
                }
            }
            self.write_ready_changed.notified().await;
        }
    }
    fn check_write(&self) -> Result<usize, StreamError> {
        let mut state = self.state();
        if let Err(e) = state.check_error() {
            return Err(e);
        }

        if state.flush_pending || state.write_budget == 0 {
            return Ok(0);
        }

        Ok(state.write_budget)
    }
    fn state(&self) -> std::sync::MutexGuard<WorkerState> {
        self.state.lock().unwrap()
    }
    fn pop(&self) -> Option<Job> {
        let mut state = self.state();
        if state.items.is_empty() {
            if state.flush_pending {
                return Some(Job::Flush);
            }
        } else if let Some(bytes) = state.items.pop_front() {
            return Some(Job::Write(bytes));
        }

        None
    }
    fn report_error(&self, e: std::io::Error) {
        {
            let mut state = self.state();
            state.alive = false;
            state.error = Some(e.into());
            state.flush_pending = false;
        }
        self.write_ready_changed.notify_one();
    }

    async fn work(&self, writer: mpsc::Sender<Bytes>) {
        loop {
            while let Some(job) = self.pop() {
                match job {
                    Job::Flush => {
                        self.state().flush_pending = false;
                    }

                    Job::Write(bytes) => {
                        tracing::debug!("worker writing: {bytes:?}");
                        let len = bytes.len();
                        match writer.send(bytes).await {
                            Err(_) => {
                                self.report_error(std::io::Error::new(
                                    std::io::ErrorKind::BrokenPipe,
                                    "Outgoing stream body reader has dropped",
                                ));
                                return;
                            }
                            Ok(_) => {
                                self.state().write_budget += len;
                            }
                        }
                    }
                }

                self.write_ready_changed.notify_one();
            }

            self.new_work.notified().await;
        }
    }
}

/// Provides a [`HostOutputStream`] impl from a [`tokio::sync::mpsc::Sender`].
pub struct BodyWriteStream {
    worker: Arc<Worker>,
    _join_handle: preview2::AbortOnDropJoinHandle<()>,
}

impl BodyWriteStream {
    /// Create a [`BodyWriteStream`].
    pub fn new(write_budget: usize, writer: mpsc::Sender<Bytes>) -> Self {
        let worker = Arc::new(Worker::new(write_budget));

        let w = Arc::clone(&worker);
        let join_handle = preview2::spawn(async move { w.work(writer).await });

        BodyWriteStream {
            worker,
            _join_handle: join_handle,
        }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for BodyWriteStream {
    fn write(&mut self, bytes: Bytes) -> Result<(), StreamError> {
        let mut state = self.worker.state();
        state.check_error()?;
        if state.flush_pending {
            return Err(StreamError::Trap(anyhow!(
                "write not permitted while flush pending"
            )));
        }
        match state.write_budget.checked_sub(bytes.len()) {
            Some(remaining_budget) => {
                state.write_budget = remaining_budget;
                state.items.push_back(bytes);
            }
            None => return Err(StreamError::Trap(anyhow!("write exceeded budget"))),
        }
        drop(state);
        self.worker.new_work.notify_one();
        Ok(())
    }
    fn flush(&mut self) -> Result<(), StreamError> {
        let mut state = self.worker.state();
        state.check_error()?;

        state.flush_pending = true;
        self.worker.new_work.notify_one();

        Ok(())
    }

    fn check_write(&mut self) -> Result<usize, StreamError> {
        self.worker.check_write()
    }
}
#[async_trait::async_trait]
impl Subscribe for BodyWriteStream {
    async fn ready(&mut self) {
        self.worker.ready().await
    }
}
