//! The `native_tls` provider.

use std::io;

use crate::{BoxFuture, TlsProvider, TlsStream, TlsTransport};

type NativeTlsStream = tokio_native_tls::TlsStream<Box<dyn TlsTransport>>;

impl crate::TlsStream for NativeTlsStream {}

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
            Ok(stream)
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
