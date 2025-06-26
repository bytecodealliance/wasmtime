//! The `rustls` provider.

use rustls::pki_types::ServerName;
use std::io;
use std::sync::{Arc, LazyLock};

use crate::{BoxFuture, TlsProvider, TlsStream, TlsTransport};

impl crate::TlsStream for tokio_rustls::client::TlsStream<Box<dyn TlsTransport>> {}

/// The `rustls` provider.
pub struct RustlsProvider {
    client_config: Arc<rustls::ClientConfig>,
}

impl TlsProvider for RustlsProvider {
    fn connect(
        &self,
        server_name: String,
        transport: Box<dyn TlsTransport>,
    ) -> BoxFuture<io::Result<Box<dyn TlsStream>>> {
        let client_config = Arc::clone(&self.client_config);
        Box::pin(async move {
            let domain = ServerName::try_from(server_name)
                .map_err(|_| io::Error::other("invalid server name"))?;

            let stream = tokio_rustls::TlsConnector::from(client_config)
                .connect(domain, transport)
                .await?;
            Ok(Box::new(stream) as Box<dyn TlsStream>)
        })
    }
}

impl Default for RustlsProvider {
    fn default() -> Self {
        static CONFIG: LazyLock<Arc<rustls::ClientConfig>> = LazyLock::new(|| {
            let roots = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.into(),
            };
            let config = rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            Arc::new(config)
        });

        Self {
            client_config: Arc::clone(&CONFIG),
        }
    }
}
