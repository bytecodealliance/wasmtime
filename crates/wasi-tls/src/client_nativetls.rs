//! A uniform TLS client interface, abstracting away the differences between the
//! `rustls` and `native-tls` implementations.

use std::io;
use tokio::io::{AsyncRead, AsyncWrite};

/// A client TLS handshake configuration object.
///
/// At the time of writing, there's nothing to configure (yet).
pub struct Handshake<IO> {
    transport: IO,
    server_name: String,
}
impl<IO> Handshake<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    /// Create a new handshake.
    pub fn new(server_name: String, transport: IO) -> Self {
        Self {
            server_name,
            transport,
        }
    }

    /// Run the handshake to completion.
    pub async fn finish(self) -> io::Result<Connection<IO>> {
        self.finish_core().await.map_err(|e| io::Error::other(e))
    }

    /// Finish the handshake, failing with a native-tls error.
    async fn finish_core(self) -> Result<Connection<IO>, native_tls::Error> {
        let connector = native_tls::TlsConnector::new()?;

        let stream = tokio_native_tls::TlsConnector::from(connector)
            .connect(&self.server_name, self.transport)
            .await?;
        Ok(stream)
    }
}

/// A TLS client connection.
pub type Connection<IO> = tokio_native_tls::TlsStream<IO>;
