#![cfg(any(feature = "rustls", feature = "openssl", feature = "nativetls"))]

#[cfg(feature = "p2")]
mod p2;
#[cfg(feature = "p3")]
mod p3;

/// Shared parts of the `*_tls_named_imports` tests.
#[cfg(any(feature = "p2", feature = "p3"))]
mod named {
    use std::pin::Pin;
    use wasmtime::component::ResourceTable;
    use wasmtime_wasi::{NamedId, WasiCtx, WasiCtxView, WasiView};
    use wasmtime_wasi_tls::{
        Error, TlsProvider, TlsStream, TlsTransport, WasiTlsCtx, WasiTlsCtxBuilder, WasiTlsCtxView,
        WasiTlsNamedView, WasiTlsView,
    };

    /// A `TlsProvider` which fails all handshakes with its own name, used to
    /// tell which context a named import used.
    struct FailingProvider(&'static str);

    impl TlsProvider for FailingProvider {
        fn connect(
            &self,
            _server_name: String,
            _transport: Box<dyn TlsTransport>,
        ) -> Pin<Box<dyn Future<Output = Result<Box<dyn TlsStream>, Error>> + Send>> {
            let err = Error::msg(format!("provider {}", self.0));
            Box::pin(async move { Err(err) })
        }
    }

    /// Store data with a WASI TLS context for imports labelled `a` and `b`.
    pub struct NamedCtx {
        table: ResourceTable,
        wasi_ctx: WasiCtx,
        default: WasiTlsCtx,
        named: [WasiTlsCtx; 2],
    }

    impl NamedCtx {
        pub fn new() -> NamedCtx {
            let ctx = |name| {
                WasiTlsCtxBuilder::new()
                    .provider(Box::new(FailingProvider(name)))
                    .build()
            };
            NamedCtx {
                table: ResourceTable::new(),
                wasi_ctx: WasiCtx::builder().inherit_stderr().build(),
                default: ctx("default"),
                named: [ctx("a"), ctx("b")],
            }
        }

        pub fn lookup(name: &str) -> NamedId {
            match name {
                "a" => NamedId(0),
                "b" => NamedId(1),
                _ => panic!("unexpected name: {name}"),
            }
        }
    }

    impl WasiView for NamedCtx {
        fn ctx(&mut self) -> WasiCtxView<'_> {
            WasiCtxView {
                ctx: &mut self.wasi_ctx,
                table: &mut self.table,
            }
        }
    }

    impl WasiTlsView for NamedCtx {
        fn tls(&mut self) -> WasiTlsCtxView<'_> {
            WasiTlsCtxView {
                ctx: &mut self.default,
                table: &mut self.table,
            }
        }
    }

    impl WasiTlsNamedView for NamedCtx {
        fn tls(&mut self, id: NamedId) -> WasiTlsCtxView<'_> {
            WasiTlsCtxView {
                ctx: &mut self.named[id.0],
                table: &mut self.table,
            }
        }
    }
}
