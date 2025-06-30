//! Auto-generated bindings.

#[expect(missing_docs, reason = "bindgen-generated code")]
mod generated {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "wasi:tls/imports",
        with: {
            "wasi:io": wasmtime_wasi::p2::bindings::io,
            "wasi:tls/types/client-connection": crate::HostClientConnection,
            "wasi:tls/types/client-handshake": crate::HostClientHandshake,
            "wasi:tls/types/future-client-streams": crate::HostFutureClientStreams,
        },
        trappable_imports: true,
        async: {
            only_imports: [],
        }
    });
}

pub use generated::wasi::tls::*;
