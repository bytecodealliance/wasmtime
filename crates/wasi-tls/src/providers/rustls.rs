//! The `rustls` provider.

use crate::{BoxFutureTlsStream, Error, TlsProvider, TlsStream, TlsTransport};
use rustls::pki_types::ServerName;
use std::sync::{Arc, LazyLock};

impl crate::TlsStream for tokio_rustls::client::TlsStream<Box<dyn TlsTransport>> {}

/// The `rustls` provider.
pub struct RustlsProvider {
    client_config: Arc<rustls::ClientConfig>,
}

impl TlsProvider for RustlsProvider {
    fn connect(&self, server_name: String, transport: Box<dyn TlsTransport>) -> BoxFutureTlsStream {
        let client_config = Arc::clone(&self.client_config);
        Box::pin(async move {
            let domain =
                ServerName::try_from(server_name).map_err(|_| Error::msg("invalid server name"))?;

            let stream = tokio_rustls::TlsConnector::from(client_config)
                .connect(domain, transport)
                .await
                .map_err(rustls_to_wasi_error)?;
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

impl From<rustls::Error> for Error {
    fn from(e: rustls::Error) -> Self {
        Error::msg(e.to_string())
    }
}

fn rustls_to_wasi_error(e: std::io::Error) -> Error {
    match e.downcast::<rustls::Error>() {
        Ok(e) => e.into(),
        Err(io_err) => Error::msg(io_err),
    }
}
