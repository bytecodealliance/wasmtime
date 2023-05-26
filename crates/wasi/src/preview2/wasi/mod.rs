pub mod command;

wasmtime::component::bindgen!({
    path: "wit",
    interfaces: "
      import wall-clock: clocks.wall-clock
      import monotonic-clock: clocks.monotonic-clock
      import timezone: clocks.timezone
      import filesystem: filesystem.filesystem
      import random: random.random
      import insecure-random: random.insecure
      import insecure-random-seed: random.insecure-seed
      import poll: poll.poll
      import streams: io.streams
      import environment: wasi-cli-base.environment
      import preopens: wasi-cli-base.preopens
      import stdin: wasi-cli-base.stdio.stdin
      import stdout: wasi-cli-base.stdio.stdout
      import stderr: wasi-cli-base.stdio.stderr
      import exit: wasi-cli-base.exit
    ",
    tracing: true,
    async: true,
    trappable_error_type: {
        "filesystem"::"error-code": Error,
        "streams"::"stream-error": Error,
    }
});
