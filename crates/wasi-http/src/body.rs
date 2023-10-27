use crate::{bindings::http::types, types::FieldMap};
use anyhow::anyhow;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use std::future::Future;
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use wasmtime_wasi::preview2::{
    self, AbortOnDropJoinHandle, HostInputStream, HostOutputStream, StreamError, Subscribe,
};

pub type HyperIncomingBody = BoxBody<Bytes, anyhow::Error>;

/// Holds onto the things needed to construct a [`HostIncomingBody`] until we are ready to build
/// one. The HostIncomingBody spawns a task that starts consuming the incoming body, and we don't
/// want to do that unless the user asks to consume the body.
pub struct HostIncomingBodyBuilder {
    pub body: HyperIncomingBody,
    pub between_bytes_timeout: Duration,
}

impl HostIncomingBodyBuilder {
    /// Consume the state held in the [`HostIncomingBodyBuilder`] to spawn a task that will drive the
    /// streaming body to completion. Data segments will be communicated out over the
    /// [`HostIncomingBodyStream`], and a [`HostFutureTrailers`] gives a way to block on/retrieve
    /// the trailers.
    pub fn build(mut self) -> HostIncomingBody {
        let (body_writer, body_receiver) = mpsc::channel(1);
        let (trailer_writer, trailers) = oneshot::channel();

        let worker = preview2::spawn(async move {
            loop {
                let frame = match tokio::time::timeout(
                    self.between_bytes_timeout,
                    http_body_util::BodyExt::frame(&mut self.body),
                )
                .await
                {
                    Ok(None) => break,

                    Ok(Some(Ok(frame))) => frame,

                    Ok(Some(Err(e))) => {
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

                    Err(_) => {
                        match body_writer
                            .send(Err(types::Error::TimeoutError(
                                "data frame timed out".to_string(),
                            )
                            .into()))
                            .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                let _ = trailer_writer.send(Err(e.0.unwrap_err()));
                            }
                        }
                        break;
                    }
                };

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

                assert!(frame.is_data(), "frame wasn't data");

                let data = frame.into_data().unwrap();

                // If the receiver no longer exists, thats ok - in that case we want to keep the
                // loop running to relieve backpressure, so we get to the trailers.
                let _ = body_writer.send(Ok(data)).await;
            }
        });

        HostIncomingBody {
            worker,
            stream: Some(HostIncomingBodyStream::new(body_receiver)),
            trailers,
        }
    }
}

pub struct HostIncomingBody {
    pub worker: AbortOnDropJoinHandle<()>,
    pub stream: Option<HostIncomingBodyStream>,
    pub trailers: oneshot::Receiver<Result<hyper::HeaderMap, anyhow::Error>>,
}

impl HostIncomingBody {
    pub fn into_future_trailers(self) -> HostFutureTrailers {
        HostFutureTrailers {
            _worker: self.worker,
            state: HostFutureTrailersState::Waiting(self.trailers),
        }
    }
}

pub struct HostIncomingBodyStream {
    pub open: bool,
    pub receiver: mpsc::Receiver<Result<Bytes, anyhow::Error>>,
    pub buffer: Bytes,
    pub error: Option<anyhow::Error>,
}

impl HostIncomingBodyStream {
    fn new(receiver: mpsc::Receiver<Result<Bytes, anyhow::Error>>) -> Self {
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
    fn read(&mut self, size: usize) -> Result<Bytes, StreamError> {
        use mpsc::error::TryRecvError;

        if !self.buffer.is_empty() {
            let len = size.min(self.buffer.len());
            let chunk = self.buffer.split_to(len);
            return Ok(chunk);
        }

        if let Some(e) = self.error.take() {
            return Err(StreamError::LastOperationFailed(e));
        }

        if !self.open {
            return Err(StreamError::Closed);
        }

        match self.receiver.try_recv() {
            Ok(Ok(mut bytes)) => {
                let len = bytes.len().min(size);
                let chunk = bytes.split_to(len);
                if !bytes.is_empty() {
                    self.buffer = bytes;
                }

                return Ok(chunk);
            }

            Ok(Err(e)) => {
                self.open = false;
                return Err(StreamError::LastOperationFailed(e));
            }

            Err(TryRecvError::Empty) => {
                return Ok(Bytes::new());
            }

            Err(TryRecvError::Disconnected) => {
                self.open = false;
                return Err(StreamError::Closed);
            }
        }
    }
}

#[async_trait::async_trait]
impl Subscribe for HostIncomingBodyStream {
    async fn ready(&mut self) {
        if !self.buffer.is_empty() {
            return;
        }

        if !self.open {
            return;
        }

        match self.receiver.recv().await {
            Some(Ok(bytes)) => self.buffer = bytes,

            Some(Err(e)) => {
                self.error = Some(e);
                self.open = false;
            }

            None => self.open = false,
        }
    }
}

pub struct HostFutureTrailers {
    _worker: AbortOnDropJoinHandle<()>,
    pub state: HostFutureTrailersState,
}

pub enum HostFutureTrailersState {
    Waiting(oneshot::Receiver<Result<hyper::HeaderMap, anyhow::Error>>),
    Done(Result<Option<FieldMap>, types::Error>),
}

#[async_trait::async_trait]
impl Subscribe for HostFutureTrailers {
    async fn ready(&mut self) {
        if let HostFutureTrailersState::Waiting(rx) = &mut self.state {
            let result = match rx.await {
                Ok(Ok(headers)) => {
                    if headers.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(headers))
                    }
                }
                Ok(Err(e)) => Err(types::Error::ProtocolError(format!("hyper error: {e:?}"))),
                Err(_) => Err(types::Error::ProtocolError(
                    "stream hung up before trailers were received".to_string(),
                )),
            };
            self.state = HostFutureTrailersState::Done(result);
        }
    }
}

pub type HyperOutgoingBody = BoxBody<Bytes, anyhow::Error>;

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
        use http_body_util::BodyExt;
        use hyper::body::{Body, Frame};
        use std::task::{Context, Poll};
        use tokio::sync::oneshot::error::RecvError;
        struct BodyImpl {
            body_receiver: mpsc::Receiver<Bytes>,
            finish_receiver: Option<oneshot::Receiver<FinishMessage>>,
        }
        impl Body for BodyImpl {
            type Data = Bytes;
            type Error = anyhow::Error;
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
                                        anyhow::anyhow!("response corrupted"),
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
