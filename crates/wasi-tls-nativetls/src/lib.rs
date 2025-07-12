//! The `native_tls` provider.

use std::{io, pin::pin};

use wasmtime_wasi_tls::{TlsProvider, TlsStream, TlsTransport};

type BoxFuture<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send>>;

/// The `native_tls` provider.
pub struct NativeTlsProvider {
    _priv: (),
}

impl TlsProvider for NativeTlsProvider {
    fn connect(
        &self,
        server_name: String,
        transport: Box<dyn TlsTransport>,
    ) -> BoxFuture<io::Result<Box<dyn TlsStream>>> {
        async fn connect_impl(
            server_name: String,
            transport: Box<dyn TlsTransport>,
        ) -> Result<NativeTlsStream, native_tls::Error> {
            let connector = native_tls::TlsConnector::new()?;
            let stream = tokio_native_tls::TlsConnector::from(connector)
                .connect(&server_name, transport)
                .await?;
            Ok(NativeTlsStream(stream))
        }

        Box::pin(async move {
            let stream = connect_impl(server_name, transport)
                .await
                .map_err(|e| io::Error::other(e))?;
            Ok(Box::new(stream) as Box<dyn TlsStream>)
        })
    }
}

impl Default for NativeTlsProvider {
    fn default() -> Self {
        Self { _priv: () }
    }
}

struct NativeTlsStream(tokio_native_tls::TlsStream<Box<dyn TlsTransport>>);

impl TlsStream for NativeTlsStream {}

impl tokio::io::AsyncRead for NativeTlsStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        pin!(&mut self.as_mut().0).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for NativeTlsStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        pin!(&mut self.as_mut().0).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        pin!(&mut self.as_mut().0).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        pin!(&mut self.as_mut().0).poll_shutdown(cx)
    }
}
