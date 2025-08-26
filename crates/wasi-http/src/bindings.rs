//! Raw bindings to the `wasi:http` package.

#[expect(missing_docs, reason = "bindgen-generated code")]
mod generated {
    use crate::body;
    use crate::types;

    wasmtime::component::bindgen!({
        path: "wit",
        world: "wasi:http/proxy",
        imports: { default: tracing | trappable },
        exports: { default: async },
        require_store_data_send: true,
        with: {
            // Upstream package dependencies
            "wasi:io": wasmtime_wasi::p2::bindings::io,

            // Configure all WIT http resources to be defined types in this
            // crate to use the `ResourceTable` helper methods.
            "wasi:http/types/outgoing-body": body::HostOutgoingBody,
            "wasi:http/types/future-incoming-response": types::HostFutureIncomingResponse,
            "wasi:http/types/outgoing-response": types::HostOutgoingResponse,
            "wasi:http/types/future-trailers": body::HostFutureTrailers,
            "wasi:http/types/incoming-body": body::HostIncomingBody,
            "wasi:http/types/incoming-response": types::HostIncomingResponse,
            "wasi:http/types/response-outparam": types::HostResponseOutparam,
            "wasi:http/types/outgoing-request": types::HostOutgoingRequest,
            "wasi:http/types/incoming-request": types::HostIncomingRequest,
            "wasi:http/types/fields": types::HostFields,
            "wasi:http/types/request-options": types::HostRequestOptions,
        },
        trappable_error_type: {
            "wasi:http/types/error-code" => crate::HttpError,
        },
    });
}

pub use self::generated::wasi::*;

/// Raw bindings to the `wasi:http/proxy` exports.
pub use self::generated::exports;

/// Bindings to the `wasi:http/proxy` world.
pub use self::generated::{LinkOptions, Proxy, ProxyIndices, ProxyPre};

/// Sync implementation of the `wasi:http/proxy` world.
pub mod sync {
    #[expect(missing_docs, reason = "bindgen-generated code")]
    mod generated {
        wasmtime::component::bindgen!({
            world: "wasi:http/proxy",
            imports: { default: tracing },
            with: {
                // http is in this crate
                "wasi:http": crate::bindings::http,
                // sync requires the wrapper in the wasmtime_wasi crate, in
                // order to have in_tokio
                "wasi:io": wasmtime_wasi::p2::bindings::sync::io,
            },
            require_store_data_send: true,
        });
    }

    pub use self::generated::wasi::*;

    /// Raw bindings to the `wasi:http/proxy` exports.
    pub use self::generated::exports;

    /// Bindings to the `wasi:http/proxy` world.
    pub use self::generated::{Proxy, ProxyIndices, ProxyPre};
}
