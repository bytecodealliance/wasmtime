//! A uniform TLS client interface, abstracting away the differences between the
//! `rustls` and `native-tls` implementations.

use rustls::pki_types::ServerName;
use std::io;
use std::sync::Arc;
use std::sync::LazyLock;
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
        let domain = ServerName::try_from(self.server_name)
            .map_err(|_| io::Error::other("invalid server name"))?;

        let stream = tokio_rustls::TlsConnector::from(Self::client_config())
            .connect(domain, self.transport)
            .await?;
        Ok(stream)
    }

    fn client_config() -> Arc<rustls::ClientConfig> {
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
}

/// A TLS client connection.
pub type Connection<IO> = tokio_rustls::client::TlsStream<IO>;
