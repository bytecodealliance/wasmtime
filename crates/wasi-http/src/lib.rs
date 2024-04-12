mod error;
mod http_impl;
mod types_impl;

pub mod body;
pub mod io;
pub mod proxy;
pub mod types;

/// Raw bindings to the `wasi:http` package.
pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        interfaces: "
            import wasi:http/incoming-handler@0.2.0;
            import wasi:http/outgoing-handler@0.2.0;
            import wasi:http/types@0.2.0;
        ",
        tracing: true,
        async: false,
        trappable_imports: true,
        with: {
            // Upstream package dependencies
            "wasi:io": wasmtime_wasi::bindings::io,

            // Configure all WIT http resources to be defined types in this
            // crate to use the `ResourceTable` helper methods.
            "wasi:http/types/outgoing-body": super::body::HostOutgoingBody,
            "wasi:http/types/future-incoming-response": super::types::HostFutureIncomingResponse,
            "wasi:http/types/outgoing-response": super::types::HostOutgoingResponse,
            "wasi:http/types/future-trailers": super::body::HostFutureTrailers,
            "wasi:http/types/incoming-body": super::body::HostIncomingBody,
            "wasi:http/types/incoming-response": super::types::HostIncomingResponse,
            "wasi:http/types/response-outparam": super::types::HostResponseOutparam,
            "wasi:http/types/outgoing-request": super::types::HostOutgoingRequest,
            "wasi:http/types/incoming-request": super::types::HostIncomingRequest,
            "wasi:http/types/fields": super::types::HostFields,
            "wasi:http/types/request-options": super::types::HostRequestOptions,
        },
        trappable_error_type: {
            "wasi:http/types/error-code" => crate::HttpError,
        },
    });

    pub use wasi::http;
}

pub use crate::error::{
    http_request_error, hyper_request_error, hyper_response_error, HttpError, HttpResult,
};
pub use crate::types::{WasiHttpCtx, WasiHttpView};
