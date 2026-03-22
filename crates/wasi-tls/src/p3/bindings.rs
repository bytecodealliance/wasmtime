//! Raw bindings to the `wasi:tls` package.

#[expect(missing_docs, reason = "generated code")]
mod generated {
    wasmtime::component::bindgen!({
        path: "src/p3/wit",
        world: "wasi:tls/imports",
        imports: {
            "wasi:tls/client.[method]connector.send": store | trappable,
            "wasi:tls/client.[method]connector.receive": store | trappable,
            default: trappable,
        },
        with: {
            "wasi:tls/client.connector": crate::p3::host::Connector,
            "wasi:tls/types.error": crate::Error,
        },
        require_store_data_send: true,
    });
}

pub use self::generated::wasi::*;
