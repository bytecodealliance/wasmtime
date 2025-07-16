//! Utility types for converting Rust & Tokio I/O types into WASI I/O types,
//! and vice versa.

use anyhow::Result;
use bytes::Bytes;
use std::io;
use std::sync::Arc;
use std::task::{Poll, ready};
use std::{future::Future, mem, pin::Pin};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;
use wasmtime_wasi::async_trait;
use wasmtime_wasi::p2::{
    DynInputStream, DynOutputStream, OutputStream, Pollable, StreamError, StreamResult,
};
use wasmtime_wasi::runtime::AbortOnDropJoinHandle;

enum FutureState<T> {
    Pending(Pin<Box<dyn Future<Output = T> + Send>>),
    Ready(T),
    Consumed,
}

pub(crate) enum FutureOutput<T> {
    Pending,
    Ready(T),
    Consumed,
}

pub(crate) struct WasiFuture<T>(FutureState<T>);

impl<T> WasiFuture<T>
where
    T: Send + 'static,
{
    pub(crate) fn spawn<F>(fut: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        Self(FutureState::Pending(Box::pin(
            wasmtime_wasi::runtime::spawn(async move { fut.await }),
        )))
    }

    pub(crate) fn get(&mut self) -> FutureOutput<T> {
        match &self.0 {
            FutureState::Pending(_) => return FutureOutput::Pending,
            FutureState::Consumed => return FutureOutput::Consumed,
            FutureState::Ready(_) => (),
        }

        let FutureState::Ready(value) = mem::replace(&mut self.0, FutureState::Consumed) else {
            unreachable!()
        };

        FutureOutput::Ready(value)
    }
}

#[async_trait]
impl<T> Pollable for WasiFuture<T>
where
    T: Send + 'static,
{
    async fn ready(&mut self) {
        match &mut self.0 {
            FutureState::Ready(_) | FutureState::Consumed => return,
            FutureState::Pending(task) => self.0 = FutureState::Ready(task.as_mut().await),
        }
    }
}

pub(crate) struct WasiStreamReader(FutureState<DynInputStream>);
impl WasiStreamReader {
    pub(crate) fn new(stream: DynInputStream) -> Self {
        Self(FutureState::Ready(stream))
    }
}
impl AsyncRead for WasiStreamReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        loop {
            let stream = match &mut self.0 {
                FutureState::Ready(stream) => stream,
                FutureState::Pending(fut) => {
                    let stream = ready!(fut.as_mut().poll(cx));
                    self.0 = FutureState::Ready(stream);
                    if let FutureState::Ready(stream) = &mut self.0 {
                        stream
                    } else {
                        unreachable!()
                    }
                }
                FutureState::Consumed => {
                    return Poll::Ready(Ok(()));
                }
            };
            match stream.read(buf.remaining()) {
                Ok(bytes) if bytes.is_empty() => {
                    let FutureState::Ready(mut stream) =
                        std::mem::replace(&mut self.0, FutureState::Consumed)
                    else {
                        unreachable!()
                    };

                    self.0 = FutureState::Pending(Box::pin(async move {
                        stream.ready().await;
                        stream
                    }));
                }
                Ok(bytes) => {
                    buf.put_slice(&bytes);

                    return Poll::Ready(Ok(()));
                }
                Err(StreamError::Closed) => {
                    self.0 = FutureState::Consumed;
                    return Poll::Ready(Ok(()));
                }
                Err(e) => {
                    self.0 = FutureState::Consumed;
                    return Poll::Ready(Err(std::io::Error::other(e)));
                }
            }
        }
    }
}

pub(crate) struct WasiStreamWriter(FutureState<DynOutputStream>);
impl WasiStreamWriter {
    pub(crate) fn new(stream: DynOutputStream) -> Self {
        Self(FutureState::Ready(stream))
    }
}
impl AsyncWrite for WasiStreamWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        loop {
            match &mut self.as_mut().0 {
                FutureState::Consumed => unreachable!(),
                FutureState::Pending(future) => {
                    let value = ready!(future.as_mut().poll(cx));
                    self.as_mut().0 = FutureState::Ready(value);
                }
                FutureState::Ready(output) => {
                    match output.check_write() {
                        Ok(0) => {
                            let FutureState::Ready(mut output) =
                                mem::replace(&mut self.as_mut().0, FutureState::Consumed)
                            else {
                                unreachable!()
                            };
                            self.as_mut().0 = FutureState::Pending(Box::pin(async move {
                                output.ready().await;
                                output
                            }));
                        }
                        Ok(count) => {
                            let count = count.min(buf.len());
                            return match output.write(Bytes::copy_from_slice(&buf[..count])) {
                                Ok(()) => Poll::Ready(Ok(count)),
                                Err(StreamError::Closed) => Poll::Ready(Ok(0)),
                                Err(e) => Poll::Ready(Err(std::io::Error::other(e))),
                            };
                        }
                        Err(StreamError::Closed) => {
                            // Our current version of tokio-rustls does not handle returning `Ok(0)` well.
                            // See: https://github.com/rustls/tokio-rustls/issues/92
                            return Poll::Ready(Err(std::io::ErrorKind::WriteZero.into()));
                        }
                        Err(e) => return Poll::Ready(Err(std::io::Error::other(e))),
                    };
                }
            }
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        self.poll_write(cx, &[]).map(|v| v.map(drop))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        self.poll_flush(cx)
    }
}

pub(crate) use wasmtime_wasi::p2::pipe::AsyncReadStream;

pub(crate) struct AsyncWriteStream<IO>(Arc<Mutex<WriteState<IO>>>);

impl<IO> AsyncWriteStream<IO>
where
    IO: AsyncWrite + Send + Unpin + 'static,
{
    pub(crate) fn new(io: IO) -> Self {
        AsyncWriteStream(Arc::new(Mutex::new(WriteState::new(io))))
    }

    pub(crate) fn close(&mut self) -> wasmtime::Result<()> {
        self.try_lock()?.close();
        Ok(())
    }

    async fn lock(&self) -> tokio::sync::MutexGuard<'_, WriteState<IO>> {
        self.0.lock().await
    }

    fn try_lock(&self) -> Result<tokio::sync::MutexGuard<'_, WriteState<IO>>, StreamError> {
        self.0
            .try_lock()
            .map_err(|_| StreamError::trap("concurrent access to resource not supported"))
    }
}
impl<IO> Clone for AsyncWriteStream<IO> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[async_trait]
impl<IO> OutputStream for AsyncWriteStream<IO>
where
    IO: AsyncWrite + Send + Unpin + 'static,
{
    fn write(&mut self, bytes: bytes::Bytes) -> StreamResult<()> {
        self.try_lock()?.write(bytes)
    }

    fn flush(&mut self) -> StreamResult<()> {
        self.try_lock()?.flush()
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        self.try_lock()?.check_write()
    }

    async fn cancel(&mut self) {
        self.lock().await.cancel().await
    }
}

#[async_trait]
impl<IO> Pollable for AsyncWriteStream<IO>
where
    IO: AsyncWrite + Send + Unpin + 'static,
{
    async fn ready(&mut self) {
        self.lock().await.ready().await
    }
}

enum WriteState<IO> {
    Ready(IO),
    Writing(AbortOnDropJoinHandle<io::Result<IO>>),
    Flushing(AbortOnDropJoinHandle<io::Result<IO>>),
    Closing(AbortOnDropJoinHandle<io::Result<()>>),
    Closed,
    Error(io::Error),
}
const READY_SIZE: usize = 1024 * 1024 * 1024;

impl<IO> WriteState<IO>
where
    IO: AsyncWrite + Send + Unpin + 'static,
{
    fn new(stream: IO) -> Self {
        Self::Ready(stream)
    }

    fn write(&mut self, mut bytes: bytes::Bytes) -> StreamResult<()> {
        let WriteState::Ready(_) = self else {
            return Err(StreamError::Trap(anyhow::anyhow!(
                "unpermitted: must call check_write first"
            )));
        };

        if bytes.is_empty() {
            return Ok(());
        }

        let WriteState::Ready(mut stream) = std::mem::replace(self, WriteState::Closed) else {
            unreachable!()
        };

        *self = WriteState::Writing(wasmtime_wasi::runtime::spawn(async move {
            while !bytes.is_empty() {
                let n = stream.write(&bytes).await?;
                let _ = bytes.split_to(n);
            }

            Ok(stream)
        }));

        Ok(())
    }

    fn flush(&mut self) -> StreamResult<()> {
        match self {
            // Immediately flush:
            WriteState::Ready(_) => {
                let WriteState::Ready(mut stream) = std::mem::replace(self, WriteState::Closed)
                else {
                    unreachable!()
                };
                *self = WriteState::Flushing(wasmtime_wasi::runtime::spawn(async move {
                    stream.flush().await?;
                    Ok(stream)
                }));
            }

            // Schedule the flush after the current write has finished:
            WriteState::Writing(_) => {
                let WriteState::Writing(write) = std::mem::replace(self, WriteState::Closed) else {
                    unreachable!()
                };
                *self = WriteState::Flushing(wasmtime_wasi::runtime::spawn(async move {
                    let mut stream = write.await?;
                    stream.flush().await?;
                    Ok(stream)
                }));
            }

            WriteState::Flushing(_) | WriteState::Closing(_) | WriteState::Error(_) => {}
            WriteState::Closed => return Err(StreamError::Closed),
        }

        Ok(())
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        match self {
            WriteState::Ready(_) => Ok(READY_SIZE),
            WriteState::Writing(_) => Ok(0),
            WriteState::Flushing(_) => Ok(0),
            WriteState::Closing(_) => Ok(0),
            WriteState::Closed => Err(StreamError::Closed),
            WriteState::Error(_) => {
                let WriteState::Error(e) = std::mem::replace(self, WriteState::Closed) else {
                    unreachable!()
                };

                Err(StreamError::LastOperationFailed(e.into()))
            }
        }
    }

    fn close(&mut self) {
        match std::mem::replace(self, WriteState::Closed) {
            // No write in progress, immediately shut down:
            WriteState::Ready(mut stream) => {
                *self = WriteState::Closing(wasmtime_wasi::runtime::spawn(async move {
                    stream.shutdown().await
                }));
            }

            // Schedule the shutdown after the current operation has finished:
            WriteState::Writing(op) | WriteState::Flushing(op) => {
                *self = WriteState::Closing(wasmtime_wasi::runtime::spawn(async move {
                    let mut stream = op.await?;
                    stream.shutdown().await
                }));
            }

            WriteState::Closing(t) => {
                *self = WriteState::Closing(t);
            }
            WriteState::Closed | WriteState::Error(_) => {}
        }
    }

    async fn cancel(&mut self) {
        match std::mem::replace(self, WriteState::Closed) {
            WriteState::Writing(task) | WriteState::Flushing(task) => _ = task.cancel().await,
            WriteState::Closing(task) => _ = task.cancel().await,
            _ => {}
        }
    }

    async fn ready(&mut self) {
        match self {
            WriteState::Writing(task) | WriteState::Flushing(task) => {
                *self = match task.await {
                    Ok(s) => WriteState::Ready(s),
                    Err(e) => WriteState::Error(e),
                }
            }
            WriteState::Closing(task) => {
                *self = match task.await {
                    Ok(()) => WriteState::Closed,
                    Err(e) => WriteState::Error(e),
                }
            }
            _ => {}
        }
    }
}
