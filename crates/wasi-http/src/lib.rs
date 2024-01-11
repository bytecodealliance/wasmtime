pub use crate::types::{WasiHttpCtx, WasiHttpView};

pub mod body;
pub mod http_impl;
pub mod io;
pub mod proxy;
pub mod types;
pub mod types_impl;

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        interfaces: "
            import wasi:http/incoming-handler@0.2.0-rc-2023-12-05;
            import wasi:http/outgoing-handler@0.2.0-rc-2023-12-05;
            import wasi:http/types@0.2.0-rc-2023-12-05;
        ",
        tracing: true,
        async: false,
        with: {
            "wasi:io/error": wasmtime_wasi::preview2::bindings::io::error,
            "wasi:io/streams": wasmtime_wasi::preview2::bindings::io::streams,
            "wasi:io/poll": wasmtime_wasi::preview2::bindings::io::poll,

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
        }
    });

    pub use wasi::http;
}

pub(crate) fn dns_error(rcode: String, info_code: u16) -> bindings::http::types::ErrorCode {
    bindings::http::types::ErrorCode::DnsError(bindings::http::types::DnsErrorPayload {
        rcode: Some(rcode),
        info_code: Some(info_code),
    })
}

pub(crate) fn internal_error(msg: String) -> bindings::http::types::ErrorCode {
    bindings::http::types::ErrorCode::InternalError(Some(msg))
}

/// Translate a [`http::Error`] to a wasi-http `ErrorCode` in the context of a request.
pub fn http_request_error(err: http::Error) -> bindings::http::types::ErrorCode {
    use bindings::http::types::ErrorCode;

    if err.is::<http::uri::InvalidUri>() {
        return ErrorCode::HttpRequestUriInvalid;
    }

    tracing::warn!("http request error: {err:?}");

    ErrorCode::HttpProtocolError
}

/// Translate a [`hyper::Error`] to a wasi-http `ErrorCode` in the context of a request.
pub fn hyper_request_error(err: hyper::Error) -> bindings::http::types::ErrorCode {
    use bindings::http::types::ErrorCode;
    use std::error::Error;

    // If there's a source, we might be able to extract a wasi-http error from it.
    if let Some(cause) = err.source() {
        if let Some(err) = cause.downcast_ref::<ErrorCode>() {
            return err.clone();
        }
    }

    tracing::warn!("hyper request error: {err:?}");

    ErrorCode::HttpProtocolError
}

/// Translate a [`hyper::Error`] to a wasi-http `ErrorCode` in the context of a response.
pub fn hyper_response_error(err: hyper::Error) -> bindings::http::types::ErrorCode {
    use bindings::http::types::ErrorCode;
    use std::error::Error;

    if err.is_timeout() {
        return ErrorCode::HttpResponseTimeout;
    }

    // If there's a source, we might be able to extract a wasi-http error from it.
    if let Some(cause) = err.source() {
        if let Some(err) = cause.downcast_ref::<ErrorCode>() {
            return err.clone();
        }
    }

    tracing::warn!("hyper response error: {err:?}");

    ErrorCode::HttpProtocolError
}
