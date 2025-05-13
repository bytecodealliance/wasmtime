//! # Wasmtime's [wasi-tls] (Transport Layer Security) Implementation
//!
//! This crate provides the Wasmtime host implementation for the [wasi-tls] API.
//! The [wasi-tls] world allows WebAssembly modules to perform SSL/TLS operations,
//! such as establishing secure connections to servers. TLS often relies on other wasi networking systems
//! to provide the stream so it will be common to enable the [wasi:cli] world as well with the networking features enabled.
//!
//! # An example of how to configure [wasi-tls] is the following:
//!
//! ```rust
//! use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};
//! use wasmtime::{
//!     component::{Linker, ResourceTable},
//!     Store, Engine, Result, Config
//! };
//! use wasmtime_wasi_tls::{LinkOptions, WasiTlsCtx};
//!
//! struct Ctx {
//!     table: ResourceTable,
//!     wasi_ctx: WasiCtx,
//! }
//!
//! impl IoView for Ctx {
//!     fn table(&mut self) -> &mut ResourceTable {
//!         &mut self.table
//!     }
//! }
//!
//! impl WasiView for Ctx {
//!     fn ctx(&mut self) -> &mut WasiCtx {
//!         &mut self.wasi_ctx
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let ctx = Ctx {
//!         table: ResourceTable::new(),
//!         wasi_ctx: WasiCtxBuilder::new()
//!             .inherit_stderr()
//!             .inherit_network()
//!             .allow_ip_name_lookup(true)
//!             .build(),
//!     };
//!
//!     let mut config = Config::new();
//!     config.async_support(true);
//!     let engine = Engine::new(&config)?;
//!
//!     // Set up wasi-cli
//!     let mut store = Store::new(&engine, ctx);
//!     let mut linker = Linker::new(&engine);
//!     wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
//!
//!     // Add wasi-tls types and turn on the feature in linker
//!     let mut opts = LinkOptions::default();
//!     opts.tls(true);
//!     wasmtime_wasi_tls::add_to_linker(&mut linker, &mut opts, |h: &mut Ctx| {
//!         WasiTlsCtx::new(&mut h.table)
//!     })?;
//!
//!     // ... use `linker` to instantiate within `store` ...
//!     Ok(())
//! }
//!
//! ```
//! [wasi-tls]: https://github.com/WebAssembly/wasi-tls
//! [wasi:cli]: https://docs.rs/wasmtime-wasi/latest

#![deny(missing_docs)]
#![doc(test(attr(deny(warnings))))]
#![doc(test(attr(allow(dead_code, unused_variables, unused_mut))))]

use anyhow::Result;
use bytes::Bytes;
use rustls::pki_types::ServerName;
use std::io;
use std::sync::Arc;
use std::task::{ready, Poll};
use std::{future::Future, mem, pin::Pin, sync::LazyLock};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio_rustls::client::TlsStream;
use wasmtime::component::{HasData, Resource, ResourceTable};
use wasmtime_wasi::async_trait;
use wasmtime_wasi::p2::bindings::io::{
    error::Error as HostIoError,
    poll::Pollable as HostPollable,
    streams::{InputStream as BoxInputStream, OutputStream as BoxOutputStream},
};
use wasmtime_wasi::p2::pipe::AsyncReadStream;
use wasmtime_wasi::p2::{OutputStream, Pollable, StreamError};
use wasmtime_wasi::runtime::AbortOnDropJoinHandle;

mod gen_ {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "wasi:tls/imports",
        with: {
            "wasi:io": wasmtime_wasi::p2::bindings::io,
            "wasi:tls/types/client-connection": super::ClientConnection,
            "wasi:tls/types/client-handshake": super::ClientHandShake,
            "wasi:tls/types/future-client-streams": super::FutureClientStreams,
        },
        trappable_imports: true,
        async: {
            only_imports: [],
        }
    });
}
pub use gen_::wasi::tls::types::LinkOptions;
use gen_::wasi::tls::{self as generated};

fn default_client_config() -> Arc<rustls::ClientConfig> {
    static CONFIG: LazyLock<Arc<rustls::ClientConfig>> = LazyLock::new(|| {
        let roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.into(),
        };
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        Arc::new(config)
    });
    Arc::clone(&CONFIG)
}

/// Wasi TLS context needed fro internal `wasi-tls`` state
pub struct WasiTlsCtx<'a> {
    table: &'a mut ResourceTable,
}

impl<'a> WasiTlsCtx<'a> {
    /// Create a new Wasi TLS context
    pub fn new(table: &'a mut ResourceTable) -> Self {
        Self { table }
    }
}

impl<'a> generated::types::Host for WasiTlsCtx<'a> {}

/// Add the `wasi-tls` world's types to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T: Send + 'static>(
    l: &mut wasmtime::component::Linker<T>,
    opts: &mut LinkOptions,
    f: fn(&mut T) -> WasiTlsCtx<'_>,
) -> Result<()> {
    generated::types::add_to_linker::<_, WasiTls>(l, &opts, f)?;
    Ok(())
}

struct WasiTls;

impl HasData for WasiTls {
    type Data<'a> = WasiTlsCtx<'a>;
}

enum TlsError {
    /// The component should trap. Under normal circumstances, this only occurs
    /// when the underlying transport stream returns [`StreamError::Trap`].
    Trap(anyhow::Error),

    /// A failure indicated by the underlying transport stream as
    /// [`StreamError::LastOperationFailed`].
    Io(wasmtime_wasi::p2::IoError),

    /// A TLS protocol error occurred.
    Tls(rustls::Error),
}

impl TlsError {
    /// Create a [`TlsError::Tls`] error from a simple message.
    fn msg(msg: &str) -> Self {
        // (Ab)using rustls' error type to synthesize our own TLS errors:
        Self::Tls(rustls::Error::General(msg.to_string()))
    }
}

impl From<io::Error> for TlsError {
    fn from(error: io::Error) -> Self {
        // Report unexpected EOFs as an error to prevent truncation attacks.
        // See: https://docs.rs/rustls/latest/rustls/struct.Reader.html#method.read
        if let io::ErrorKind::WriteZero | io::ErrorKind::UnexpectedEof = error.kind() {
            return Self::msg("underlying transport closed abruptly");
        }

        // Errors from underlying transport.
        // These have been wrapped inside `io::Error`s by our wasi-to-tokio stream transformer below.
        let error = match error.downcast::<StreamError>() {
            Ok(StreamError::LastOperationFailed(e)) => return Self::Io(e),
            Ok(StreamError::Trap(e)) => return Self::Trap(e),
            Ok(StreamError::Closed) => unreachable!("our wasi-to-tokio stream transformer should have translated this to a 0-sized read"),
            Err(e) => e,
        };

        // Errors from `rustls`.
        // These have been wrapped inside `io::Error`s by `tokio-rustls`.
        let error = match error.downcast::<rustls::Error>() {
            Ok(e) => return Self::Tls(e),
            Err(e) => e,
        };

        // All errors should have been handled by the clauses above.
        Self::Trap(anyhow::Error::new(error).context("unknown wasi-tls error"))
    }
}

///  Represents the ClientHandshake which will be used to configure the handshake
pub struct ClientHandShake {
    server_name: String,
    streams: WasiStreams,
}

impl<'a> generated::types::HostClientHandshake for WasiTlsCtx<'a> {
    fn new(
        &mut self,
        server_name: String,
        input: Resource<BoxInputStream>,
        output: Resource<BoxOutputStream>,
    ) -> wasmtime::Result<Resource<ClientHandShake>> {
        let input = self.table.delete(input)?;
        let output = self.table.delete(output)?;
        Ok(self.table.push(ClientHandShake {
            server_name,
            streams: WasiStreams {
                input: StreamState::Ready(input),
                output: StreamState::Ready(output),
            },
        })?)
    }

    fn finish(
        &mut self,
        this: wasmtime::component::Resource<ClientHandShake>,
    ) -> wasmtime::Result<Resource<FutureClientStreams>> {
        let handshake = self.table.delete(this)?;
        let server_name = handshake.server_name;
        let streams = handshake.streams;

        Ok(self
            .table
            .push(FutureStreams(StreamState::Pending(Box::pin(async move {
                let domain = ServerName::try_from(server_name)
                    .map_err(|_| TlsError::msg("invalid server name"))?;

                let stream = tokio_rustls::TlsConnector::from(default_client_config())
                    .connect(domain, streams)
                    .await?;
                Ok(stream)
            }))))?)
    }

    fn drop(
        &mut self,
        this: wasmtime::component::Resource<ClientHandShake>,
    ) -> wasmtime::Result<()> {
        self.table.delete(this)?;
        Ok(())
    }
}

/// Future streams provides the tls streams after the handshake is completed
pub struct FutureStreams<T>(StreamState<Result<T, TlsError>>);

/// Library specific version of TLS connection after the handshake is completed.
/// This alias allows it to use with wit-bindgen component generator which won't take generic types
pub type FutureClientStreams = FutureStreams<TlsStream<WasiStreams>>;

#[async_trait]
impl<T: Send + 'static> Pollable for FutureStreams<T> {
    async fn ready(&mut self) {
        match &mut self.0 {
            StreamState::Ready(_) | StreamState::Closed => return,
            StreamState::Pending(task) => self.0 = StreamState::Ready(task.as_mut().await),
        }
    }
}

impl<'a> generated::types::HostFutureClientStreams for WasiTlsCtx<'a> {
    fn subscribe(
        &mut self,
        this: wasmtime::component::Resource<FutureClientStreams>,
    ) -> wasmtime::Result<Resource<HostPollable>> {
        wasmtime_wasi::p2::subscribe(self.table, this)
    }

    fn get(
        &mut self,
        this: wasmtime::component::Resource<FutureClientStreams>,
    ) -> wasmtime::Result<
        Option<
            Result<
                Result<
                    (
                        Resource<ClientConnection>,
                        Resource<BoxInputStream>,
                        Resource<BoxOutputStream>,
                    ),
                    Resource<HostIoError>,
                >,
                (),
            >,
        >,
    > {
        let this = &mut self.table.get_mut(&this)?.0;
        match this {
            StreamState::Pending(_) => return Ok(None),
            StreamState::Closed => return Ok(Some(Err(()))),
            StreamState::Ready(_) => (),
        }

        let StreamState::Ready(result) = mem::replace(this, StreamState::Closed) else {
            unreachable!()
        };

        let tls_stream = match result {
            Ok(s) => s,
            Err(TlsError::Trap(e)) => return Err(e),
            Err(TlsError::Io(e)) => {
                let error = self.table.push(e)?;
                return Ok(Some(Ok(Err(error))));
            }
            Err(TlsError::Tls(e)) => {
                let error = self.table.push(wasmtime_wasi::p2::IoError::new(e))?;
                return Ok(Some(Ok(Err(error))));
            }
        };

        let (rx, tx) = tokio::io::split(tls_stream);
        let write_stream = AsyncTlsWriteStream::new(TlsWriter::new(tx));
        let client = ClientConnection {
            writer: write_stream.clone(),
        };

        let input = Box::new(AsyncReadStream::new(rx)) as BoxInputStream;
        let output = Box::new(write_stream) as BoxOutputStream;

        let client = self.table.push(client)?;
        let input = self.table.push_child(input, &client)?;
        let output = self.table.push_child(output, &client)?;

        Ok(Some(Ok(Ok((client, input, output)))))
    }

    fn drop(
        &mut self,
        this: wasmtime::component::Resource<FutureClientStreams>,
    ) -> wasmtime::Result<()> {
        self.table.delete(this)?;
        Ok(())
    }
}

/// Represents the client connection and used to shut down the tls stream
pub struct ClientConnection {
    writer: AsyncTlsWriteStream,
}

impl<'a> generated::types::HostClientConnection for WasiTlsCtx<'a> {
    fn close_output(&mut self, this: Resource<ClientConnection>) -> wasmtime::Result<()> {
        self.table.get_mut(&this)?.writer.close()
    }

    fn drop(&mut self, this: Resource<ClientConnection>) -> wasmtime::Result<()> {
        self.table.delete(this)?;
        Ok(())
    }
}

enum StreamState<T> {
    Ready(T),
    Pending(Pin<Box<dyn Future<Output = T> + Send>>),
    Closed,
}

/// Wrapper around Input and Output wasi IO Stream that provides Async Read/Write
pub struct WasiStreams {
    input: StreamState<BoxInputStream>,
    output: StreamState<BoxOutputStream>,
}

impl AsyncWrite for WasiStreams {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        loop {
            match &mut self.as_mut().output {
                StreamState::Closed => unreachable!(),
                StreamState::Pending(future) => {
                    let value = ready!(future.as_mut().poll(cx));
                    self.as_mut().output = StreamState::Ready(value);
                }
                StreamState::Ready(output) => {
                    match output.check_write() {
                        Ok(0) => {
                            let StreamState::Ready(mut output) =
                                mem::replace(&mut self.as_mut().output, StreamState::Closed)
                            else {
                                unreachable!()
                            };
                            self.as_mut().output = StreamState::Pending(Box::pin(async move {
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

impl AsyncRead for WasiStreams {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        loop {
            let stream = match &mut self.input {
                StreamState::Ready(stream) => stream,
                StreamState::Pending(fut) => {
                    let stream = ready!(fut.as_mut().poll(cx));
                    self.input = StreamState::Ready(stream);
                    if let StreamState::Ready(stream) = &mut self.input {
                        stream
                    } else {
                        unreachable!()
                    }
                }
                StreamState::Closed => {
                    return Poll::Ready(Ok(()));
                }
            };
            match stream.read(buf.remaining()) {
                Ok(bytes) if bytes.is_empty() => {
                    let StreamState::Ready(mut stream) =
                        std::mem::replace(&mut self.input, StreamState::Closed)
                    else {
                        unreachable!()
                    };

                    self.input = StreamState::Pending(Box::pin(async move {
                        stream.ready().await;
                        stream
                    }));
                }
                Ok(bytes) => {
                    buf.put_slice(&bytes);

                    return Poll::Ready(Ok(()));
                }
                Err(StreamError::Closed) => {
                    self.input = StreamState::Closed;
                    return Poll::Ready(Ok(()));
                }
                Err(e) => {
                    self.input = StreamState::Closed;
                    return Poll::Ready(Err(std::io::Error::other(e)));
                }
            }
        }
    }
}

type TlsWriteHalf = tokio::io::WriteHalf<tokio_rustls::client::TlsStream<WasiStreams>>;

struct TlsWriter {
    state: WriteState,
}

enum WriteState {
    Ready(TlsWriteHalf),
    Writing(AbortOnDropJoinHandle<io::Result<TlsWriteHalf>>),
    Closing(AbortOnDropJoinHandle<io::Result<()>>),
    Closed,
    Error(io::Error),
}
const READY_SIZE: usize = 1024 * 1024 * 1024;

impl TlsWriter {
    fn new(stream: TlsWriteHalf) -> Self {
        Self {
            state: WriteState::Ready(stream),
        }
    }

    fn write(&mut self, mut bytes: bytes::Bytes) -> Result<(), StreamError> {
        let WriteState::Ready(_) = self.state else {
            return Err(StreamError::Trap(anyhow::anyhow!(
                "unpermitted: must call check_write first"
            )));
        };

        if bytes.is_empty() {
            return Ok(());
        }

        let WriteState::Ready(mut stream) = std::mem::replace(&mut self.state, WriteState::Closed)
        else {
            unreachable!()
        };

        self.state = WriteState::Writing(wasmtime_wasi::runtime::spawn(async move {
            while !bytes.is_empty() {
                match stream.write(&bytes).await {
                    Ok(n) => {
                        let _ = bytes.split_to(n);
                    }
                    Err(e) => return Err(e.into()),
                }
            }

            Ok(stream)
        }));

        Ok(())
    }

    fn flush(&mut self) -> Result<(), StreamError> {
        // `flush` is a no-op here, as we're not managing any internal buffer.
        match self.state {
            WriteState::Ready(_)
            | WriteState::Writing(_)
            | WriteState::Closing(_)
            | WriteState::Error(_) => Ok(()),
            WriteState::Closed => Err(StreamError::Closed),
        }
    }

    fn check_write(&mut self) -> Result<usize, StreamError> {
        match &mut self.state {
            WriteState::Ready(_) => Ok(READY_SIZE),
            WriteState::Writing(_) => Ok(0),
            WriteState::Closing(_) => Ok(0),
            WriteState::Closed => Err(StreamError::Closed),
            WriteState::Error(_) => {
                let WriteState::Error(e) = std::mem::replace(&mut self.state, WriteState::Closed)
                else {
                    unreachable!()
                };

                Err(StreamError::LastOperationFailed(e.into()))
            }
        }
    }

    fn close(&mut self) {
        match std::mem::replace(&mut self.state, WriteState::Closed) {
            // No write in progress, immediately shut down:
            WriteState::Ready(mut stream) => {
                self.state = WriteState::Closing(wasmtime_wasi::runtime::spawn(async move {
                    stream.shutdown().await
                }));
            }

            // Schedule the shutdown after the current write has finished:
            WriteState::Writing(write) => {
                self.state = WriteState::Closing(wasmtime_wasi::runtime::spawn(async move {
                    let mut stream = write.await?;
                    stream.shutdown().await
                }));
            }

            WriteState::Closing(t) => {
                self.state = WriteState::Closing(t);
            }
            WriteState::Closed | WriteState::Error(_) => {}
        }
    }

    async fn cancel(&mut self) {
        match std::mem::replace(&mut self.state, WriteState::Closed) {
            WriteState::Writing(task) => _ = task.cancel().await,
            WriteState::Closing(task) => _ = task.cancel().await,
            _ => {}
        }
    }

    async fn ready(&mut self) {
        match &mut self.state {
            WriteState::Writing(task) => {
                self.state = match task.await {
                    Ok(s) => WriteState::Ready(s),
                    Err(e) => WriteState::Error(e),
                }
            }
            WriteState::Closing(task) => {
                self.state = match task.await {
                    Ok(()) => WriteState::Closed,
                    Err(e) => WriteState::Error(e),
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
struct AsyncTlsWriteStream(Arc<Mutex<TlsWriter>>);

impl AsyncTlsWriteStream {
    fn new(writer: TlsWriter) -> Self {
        AsyncTlsWriteStream(Arc::new(Mutex::new(writer)))
    }

    fn close(&mut self) -> wasmtime::Result<()> {
        try_lock_for_stream(&self.0)?.close();
        Ok(())
    }
}

#[async_trait]
impl OutputStream for AsyncTlsWriteStream {
    fn write(&mut self, bytes: bytes::Bytes) -> Result<(), StreamError> {
        try_lock_for_stream(&self.0)?.write(bytes)
    }

    fn flush(&mut self) -> Result<(), StreamError> {
        try_lock_for_stream(&self.0)?.flush()
    }

    fn check_write(&mut self) -> Result<usize, StreamError> {
        try_lock_for_stream(&self.0)?.check_write()
    }

    async fn cancel(&mut self) {
        self.0.lock().await.cancel().await
    }
}

#[async_trait]
impl Pollable for AsyncTlsWriteStream {
    async fn ready(&mut self) {
        self.0.lock().await.ready().await
    }
}

fn try_lock_for_stream<TlsWriter>(
    mutex: &Mutex<TlsWriter>,
) -> Result<tokio::sync::MutexGuard<'_, TlsWriter>, StreamError> {
    mutex
        .try_lock()
        .map_err(|_| StreamError::trap("concurrent access to resource not supported"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn test_future_client_streams_ready_can_be_canceled() {
        let (tx1, rx1) = oneshot::channel::<()>();

        let mut future_streams = FutureStreams(StreamState::Pending(Box::pin(async move {
            rx1.await
                .map_err(|_| TlsError::Trap(anyhow::anyhow!("oneshot canceled")))
        })));

        let mut fut = future_streams.ready();

        let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());
        assert!(fut.as_mut().poll(&mut cx).is_pending());

        //cancel the readiness check
        drop(fut);

        match future_streams.0 {
            StreamState::Closed => panic!("First future should be in Pending/ready state"),
            _ => (),
        }

        // make it ready and wait for it to progress
        tx1.send(()).unwrap();
        future_streams.ready().await;

        match future_streams.0 {
            StreamState::Ready(Ok(())) => (),
            _ => panic!("First future should be in Ready(Err) state"),
        }
    }
}
