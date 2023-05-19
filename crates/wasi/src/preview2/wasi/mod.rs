pub mod command;

wasmtime::component::bindgen!({
    path: "wit",
    interfaces: "
      import wall-clock: clocks.wall-clock
      import monotonic-clock: clocks.monotonic-clock
      import timezone: clocks.timezone
      import filesystem: filesystem.filesystem
      import random: random.random
      import poll: poll.poll
      import streams: io.streams
      import environment: wasi-cli-base.environment
      import preopens: wasi-cli-base.preopens
      import exit: wasi-cli-base.exit
    ",
    tracing: true,
    async: true,
    trappable_error_type: {
        "filesystem"::"error-code": Error,
        "streams"::"stream-error": Error,
    }
});
