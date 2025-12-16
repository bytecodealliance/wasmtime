//! Raw bindings to the `wasi:tls` package.

#[expect(missing_docs, reason = "generated code")]
mod generated {
    wasmtime::component::bindgen!({
        path: "src/p3/wit",
        world: "wasi:tls/imports",
        imports: {
            "wasi:tls/client.[static]handshake.finish": trappable | tracing | store,
            "wasi:tls/client.connect": trappable | tracing | store,
            "wasi:tls/server.[static]handshake.finish": trappable | tracing | store,
            default: trappable | tracing
        },
        with: {
            "wasi:tls/client.handshake": crate::p3::ClientHandshake,
            "wasi:tls/client.hello": crate::p3::ClientHello,
            "wasi:tls/server.handshake": crate::p3::ServerHandshake,
            "wasi:tls/types.certificate": crate::p3::Certificate,
        },
    });
}

pub use self::generated::wasi::*;
