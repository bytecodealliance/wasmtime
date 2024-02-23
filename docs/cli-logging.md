# Logging in the `wasmtime` CLI

Wasmtime's libraries use Rust's [`log`] crate to log diagnostic
information, and the `wasmtime` CLI executable uses [`tracing-subscriber`]
by default for logging this information to the console.

Basic logging is controlled by the `WASMTIME_LOG` environment variable. For
example, To enable logging of WASI system calls, similar to the `strace`
command on Linux, set `WASMTIME_LOG=wasmtime_wasi`.

```sh
$ WASMTIME_LOG=wasmtime_wasi wasmtime hello.wasm
[...]
2024-02-23T01:50:18.751421Z TRACE wiggle abi{module="wasi_snapshot_preview1" function="fd_write"}: wasmtime_wasi::preview2::preview1::wasi_snapshot_preview1: fd=Fd(1) iovs=*guest 0xffea0/1
Hello, world!
2024-02-23T01:50:18.751479Z TRACE wiggle abi{module="wasi_snapshot_preview1" function="fd_write"}: wasmtime_wasi::preview2::preview1::wasi_snapshot_preview1: result=Ok(6)
```

Wasmtime can also redirect the log messages into log files, with the `-D
log-to-files` option. It creates one file per thread within Wasmtime, with the
files named `wasmtime.dbg.*`.

[`log`]: https://crates.io/crates/log
[`tracing-subscriber`]: https://crates.io/crates/tracing-subscriber
