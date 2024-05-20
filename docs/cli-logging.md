# Logging in the `wasmtime` CLI

Wasmtime's libraries use Rust's [`log`] crate to log diagnostic
information, and the `wasmtime` CLI executable uses [`pretty_env_logger`]
by default for logging this information to the console.

Basic logging is controlled by the `WASMTIME_LOG` environment variable. For example,
To enable logging of WASI system calls, similar to the `strace` command on Linux,
set `WASMTIME_LOG=wasmtime_wasi=trace`.

```sh
$ WASMTIME_LOG=wasmtime_wasi=trace hello.wasm
[...]
TRACE wiggle abi{module="wasi_snapshot_preview1" function="fd_write"} wasmtime_wasi::preview1::wasi_snapshot_preview1                     > fd=Fd(1) iovs=*guest 0x14/1
Hello, world!
TRACE wiggle abi{module="wasi_snapshot_preview1" function="fd_write"}: wasmtime_wasi::preview1::wasi_snapshot_preview1: result=Ok(14)
TRACE wiggle abi{module="wasi_snapshot_preview1" function="proc_exit"}: wasmtime_wasi::preview1::wasi_snapshot_preview1: rval=1
TRACE wiggle abi{module="wasi_snapshot_preview1" function="proc_exit"}: wasmtime_wasi::preview1::wasi_snapshot_preview1: result=Exited with i32 exit status 1
```

Wasmtime can also redirect the log messages into log files, with the
`-D log-to-files` option. It creates one file per thread within Wasmtime, with
the files named `wasmtime.dbg.*`.

[`log`]: https://crates.io/crates/log
[`pretty_env_logger`]: https://crates.io/crates/pretty_env_logger
