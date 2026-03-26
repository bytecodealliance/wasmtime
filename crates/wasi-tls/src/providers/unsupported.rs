//! The `unsupported` provider.

use crate::{BoxFutureTlsStream, Error, TlsProvider, TlsTransport};

/// A pseudo TLS provider that returns an error for all operations. This is the
/// default provider when no real TLS providers were enabled at compile time.
#[derive(Default)]
pub struct UnsupportedProvider {
    _priv: (),
}

impl TlsProvider for UnsupportedProvider {
    fn connect(
        &self,
        _server_name: String,
        _transport: Box<dyn TlsTransport>,
    ) -> BoxFutureTlsStream {
        Box::pin(async move {
            Err(Error::msg(
                "no TLS provider enabled; recompile with a TLS provider feature",
            ))
        })
    }
}
