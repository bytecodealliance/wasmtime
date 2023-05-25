pub mod command;

wasmtime::component::bindgen!({
    path: "wit",
    interfaces: "
      import wasi:clocks/wall-clock
      import wasi:clocks/monotonic-clock
      import wasi:clocks/timezone
      import wasi:filesystem/filesystem
      import wasi:random/random
      import wasi:poll/poll
      import wasi:io/streams
      import wasi:cli-base/environment
      import wasi:cli-base/preopens
      import wasi:cli-base/exit
    ",
    tracing: true,
    async: true,
    trappable_error_type: {
        "filesystem"::"error-code": Error,
        "streams"::"stream-error": Error,
    }
});

pub use wasi::*;
