//! Raw bindings to the `wasi:http` package.

#[expect(missing_docs, reason = "generated code")]
mod generated {
    wasmtime::component::bindgen!({
        path: "src/p3/wit",
        world: "wasi:http/proxy",
        imports: {
            "wasi:http/handler.handle": store | trappable | tracing,
            "wasi:http/types.[drop]request": store | trappable | tracing,
            "wasi:http/types.[drop]response": store | trappable | tracing,
            "wasi:http/types.[static]request.consume-body": store | trappable | tracing,
            "wasi:http/types.[static]request.new": store | trappable | tracing,
            "wasi:http/types.[static]response.consume-body": store | trappable | tracing,
            "wasi:http/types.[static]response.new": store | trappable | tracing,
            default: trappable | tracing,
        },
        exports: { default: async | store | task_exit },
        with: {
            "wasi:http/types.fields": with::Fields,
            "wasi:http/types.request": crate::p3::Request,
            "wasi:http/types.request-options": with::RequestOptions,
            "wasi:http/types.response": crate::p3::Response,
        },
        trappable_error_type: {
            "wasi:http/types.error-code" => crate::p3::HttpError,
            "wasi:http/types.header-error" => crate::p3::HeaderError,
            "wasi:http/types.request-options-error" => crate::p3::RequestOptionsError,
        },
    });

    mod with {
        pub type Fields = crate::p3::MaybeMutable<http::HeaderMap>;
        pub type RequestOptions = crate::p3::MaybeMutable<crate::p3::RequestOptions>;
    }
}

pub use self::generated::wasi::*;

/// Raw bindings to the `wasi:http/proxy` exports.
pub use self::generated::exports;

/// Bindings to the `wasi:http/proxy` world.
pub use self::generated::{Proxy, ProxyIndices, ProxyPre};
