//! The `native_tls` provider.

use crate::{BoxFutureTlsStream, Error, TlsProvider, TlsStream, TlsTransport};
use std::io;
use std::pin::{Pin, pin};
use std::task::Poll;

/// The `native_tls` provider.
pub struct NativeTlsProvider {
    _priv: (),
}

impl TlsProvider for NativeTlsProvider {
    fn connect(&self, server_name: String, transport: Box<dyn TlsTransport>) -> BoxFutureTlsStream {
        Box::pin(async move {
            let connector = native_tls::TlsConnector::new()?;
            let stream = tokio_native_tls::TlsConnector::from(connector)
                .connect(&server_name, transport)
                .await?;
            Ok(Box::new(NativeTlsStream(stream)) as Box<dyn TlsStream>)
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
    ) -> Poll<io::Result<()>> {
        pin!(&mut self.as_mut().0).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for NativeTlsStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        pin!(&mut self.as_mut().0).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        pin!(&mut self.as_mut().0).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        match pin!(&mut self.as_mut().0).poll_shutdown(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => return Poll::Pending,
        }

        // `tokio-rustls` & `tokio-openssl` shut down the underlying transport,
        // but `tokio-native-tls` does not, so we need to do that ourselves:
        let inner = self.0.get_mut().get_mut().get_mut();
        Pin::new(inner).poll_shutdown(cx)
    }
}

impl From<native_tls::Error> for Error {
    fn from(e: native_tls::Error) -> Self {
        Error::msg(e.to_string())
    }
}
