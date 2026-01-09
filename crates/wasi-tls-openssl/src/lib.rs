//! The `openssl` provider.

use openssl::ssl::{SslConnector, SslMethod};
use std::{
    io,
    pin::{Pin, pin},
};
use wasmtime_wasi_tls::{TlsProvider, TlsStream, TlsTransport};

type BoxFuture<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send>>;

/// The `openssl` provider.
pub struct OpenSslProvider {
    _priv: (),
}

impl TlsProvider for OpenSslProvider {
    fn connect(
        &self,
        server_name: String,
        transport: Box<dyn TlsTransport>,
    ) -> BoxFuture<io::Result<Box<dyn TlsStream>>> {
        async fn connect_impl(
            server_name: String,
            transport: Box<dyn TlsTransport>,
        ) -> Result<OpenSslStream, openssl::ssl::Error> {
            // Per the `openssl` crate's recommendation, we're using the
            // `SslConnector` to set up a Ssl object with secure defaults:
            //
            // https://docs.rs/openssl/latest/openssl/ssl/struct.SslConnector.html
            // > OpenSSL's default configuration is highly insecure. This
            // > connector manages the OpenSSL structures, configuring cipher
            // > suites, session options, hostname verification, and more.
            let config = SslConnector::builder(SslMethod::tls_client())?
                .build()
                .configure()?;
            let ssl = config.into_ssl(&server_name)?;
            let mut stream = tokio_openssl::SslStream::new(ssl, transport)?;
            Pin::new(&mut stream).connect().await?;
            Ok(OpenSslStream(stream))
        }

        Box::pin(async move {
            let stream = connect_impl(server_name, transport)
                .await
                .map_err(|e| io::Error::other(e))?;
            Ok(Box::new(stream) as Box<dyn TlsStream>)
        })
    }
}

impl Default for OpenSslProvider {
    fn default() -> Self {
        Self { _priv: () }
    }
}

struct OpenSslStream(tokio_openssl::SslStream<Box<dyn TlsTransport>>);

impl TlsStream for OpenSslStream {}

impl tokio::io::AsyncRead for OpenSslStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        pin!(&mut self.as_mut().0).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for OpenSslStream {
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
