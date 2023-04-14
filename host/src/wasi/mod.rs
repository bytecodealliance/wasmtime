pub mod command;
pub mod proxy;

wasmtime::component::bindgen!({
    // path: "../wit",
    // This is a union of the imports in the command and proxy worlds:
    interfaces: "
      import wall-clock: clocks.wall-clock
      import monotonic-clock: clocks.monotonic-clock
      import timezone: clocks.timezone
      import filesystem: filesystem.filesystem
      import instance-network: sockets.instance-network
      import ip-name-lookup: sockets.ip-name-lookup
      import network: sockets.network
      import tcp-create-socket: sockets.tcp-create-socket
      import tcp: sockets.tcp
      import udp-create-socket: sockets.udp-create-socket
      import udp: sockets.udp
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
