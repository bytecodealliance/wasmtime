pub mod command;

pub mod sync_io {
    pub(crate) mod _internal {
        wasmtime::component::bindgen!({
            path: "wit",
            interfaces: "
              import wasi:poll/poll
              import wasi:io/streams
            ",
            tracing: true,
            trappable_error_type: {
                "streams"::"stream-error": Error,
            }
        });
    }
    pub use self::_internal::wasi::{io, poll};
}

pub(crate) mod _internal_io {
    wasmtime::component::bindgen!({
        path: "wit",
        interfaces: "
              import wasi:poll/poll
              import wasi:io/streams
            ",
        tracing: true,
        async: true,
        trappable_error_type: {
            "streams"::"stream-error": Error,
        }
    });
}
pub use self::_internal_io::wasi::{io, poll};
pub(crate) mod _internal_rest {
    wasmtime::component::bindgen!({
    path: "wit",
    interfaces: "
              import wasi:clocks/wall-clock
              import wasi:clocks/monotonic-clock
              import wasi:clocks/timezone
              import wasi:filesystem/filesystem
              import wasi:random/random
              import wasi:random/insecure
              import wasi:random/insecure-seed
              import wasi:cli-base/environment
              import wasi:cli-base/preopens
              import wasi:cli-base/exit
              import wasi:cli-base/stdin
              import wasi:cli-base/stdout
              import wasi:cli-base/stderr
            ",
    tracing: true,
    trappable_error_type: {
        "filesystem"::"error-code": Error,
        "streams"::"stream-error": Error,
    },
    with: {
       "wasi:poll/poll": crate::preview2::wasi::poll::poll,
       "wasi:io/streams": crate::preview2::wasi::io::streams
    }
    });
}
pub use self::_internal_rest::wasi::*;
