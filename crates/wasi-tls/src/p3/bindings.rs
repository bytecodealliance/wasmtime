//! Raw bindings to the `wasi:tls` package.

#[expect(missing_docs, reason = "generated code")]
mod generated {
    wasmtime::component::bindgen!({
        path: "src/p3/wit",
        world: "wasi:tls/imports",
        imports: {
            "wasi:tls/client.[method]connector.receive": trappable | tracing | store,
            "wasi:tls/client.[method]connector.send": trappable | tracing | store,
            default: trappable | tracing
        },
        with: {
            "wasi:tls/client.connector": crate::p3::Connector,
            "wasi:tls/types.error": String,
        },
    });
}

pub use self::generated::wasi::*;
