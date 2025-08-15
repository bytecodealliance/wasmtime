//! Raw bindings to the `wasi:http` package.

#[expect(missing_docs, reason = "generated code")]
mod generated {
    wasmtime::component::bindgen!({
        path: "src/p3/wit",
        world: "wasi:http/proxy",
        imports: {
            "wasi:http/handler/[async]handle": async | store | trappable | tracing,
            "wasi:http/types/[static]request.new": async | store | trappable | tracing,
            "wasi:http/types/[static]response.new": async | store | trappable | tracing,
            default: trappable | tracing,
        },
        exports: { default: async | store },
    });
}

pub use self::generated::wasi::*;

/// Raw bindings to the `wasi:http/proxy` exports.
pub use self::generated::exports;

/// Bindings to the `wasi:http/proxy` world.
pub use self::generated::{Proxy, ProxyIndices, ProxyPre};
