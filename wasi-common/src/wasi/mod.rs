pub mod command;
pub mod proxy;

wasmtime::component::bindgen!({
    path: "../wit",
    interfaces: "
      import wall-clock: clocks.wall-clock
      import monotonic-clock: clocks.monotonic-clock
      import timezone: clocks.timezone
      import filesystem: filesystem.filesystem
      import random: random.random
      import poll: poll.poll
      import streams: io.streams
      import environment: wasi-base.environment
      import preopens: wasi-base.preopens
      import exit: wasi-base.exit
      import console: logging.handler
      import default-outgoing-HTTP: http.outgoing-handler
    ",
    tracing: true,
    async: true,
    trappable_error_type: {
        "filesystem"::"error-code": Error,
        "streams"::"stream-error": Error,
    }
});
