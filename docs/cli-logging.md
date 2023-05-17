# Logging in the `wasmtime` CLI

Wasmtime's libraries use Rust's [`log`] crate to log diagnostic
information, and the `wasmtime` CLI executable uses [`pretty_env_logger`]
by default for logging this information to the console.

Basic logging is controlled by the `RUST_LOG` environment variable. For example,
To enable logging of WASI system calls, similar to the `strace` command on Linux,
set `RUST_LOG=wasi_common=trace`.

```sh
$ RUST_LOG=wasi_common=trace wasmtime hello.wasm
[...]
 TRACE wasi_common::hostcalls_impl::fs                       > fd_write(fd=1, iovs_ptr=0x10408, iovs_len=1, nwritten=0x10404)
Hello, world!
 TRACE wasi_common::hostcalls_impl::fs                       >      | *nwritten=14
 TRACE wasi_common::hostcalls                                >      | errno=ESUCCESS (No error occurred. System call completed successfully.)
 TRACE wasi_common::hostcalls_impl::misc                     > proc_exit(rval=1)
```

Wasmtime can also redirect the log messages into log files, with the
`--log-to-files` option. It creates one file per thread within Wasmtime, with
the files named `wasmtime.dbg.*`.

[`log`]: https://crates.io/crates/log
[`pretty_env_logger`]: https://crates.io/crates/pretty_env_logger
