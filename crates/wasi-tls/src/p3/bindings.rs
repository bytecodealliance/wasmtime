//! Raw bindings to the `wasi:tls` package.

#[expect(missing_docs, reason = "generated code")]
mod generated {
    wasmtime::component::bindgen!({
        path: "src/p3/wit",
        world: "wasi:tls/imports",
        require_store_data_send: true,
    });
}

pub use self::generated::wasi::*;
