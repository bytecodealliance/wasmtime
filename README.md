# wasi-common

This repo strips away those bits of [lucet-wasi](https://github.com/fastly/lucet/tree/5d3efb6005391a7c71d585732a5507b00db6bb1e/lucet-wasi)
which can potentially be encapsulated in a separated crate with potential plug'n'play use in both
[Lucet](https://github.com/fastly/lucet)
and [Wasmtime](https://github.com/CraneStation/wasmtime) projects.

This repo is strictly experimental, and based on [5d3efb6005](https://github.com/fastly/lucet/commit/5d3efb6005391a7c71d585732a5507b00db6bb1e) git revision.

## Supported syscalls

We support a subset of the [WASI
API](https://github.com/CraneStation/wasmtime/blob/master/docs/WASI-api.md), though we are adding
new syscalls on a regular basis. We currently implement:

- `__wasi_args_get`
- `__wasi_args_sizes_get`
- `__wasi_clock_res_get`
- `__wasi_clock_time_get`
- `__wasi_environ_get`
- `__wasi_environ_sizes_get`
- `__wasi_fd_close`
- `__wasi_fd_fdstat_get`
- `__wasi_fd_fdstat_set_flags`
- `__wasi_fd_prestat_dir_name`
- `__wasi_fd_prestat_get`
- `__wasi_fd_read`
- `__wasi_fd_seek`
- `__wasi_fd_write`
- `__wasi_path_open`
- `__wasi_proc_exit`
- `__wasi_random_get`

This is enough to run basic C and Rust programs, including those that use command-line arguments,
environment variables, stdio, and basic file operations.

## Third-Party Code

`src/wasm32.rs` is copied from
[wasmtime](https://github.com/CraneStation/wasmtime/blob/master/wasmtime-wasi/src/wasm32.rs), along
with the associated `LICENSE.wasmtime` file.

Significant parts of our syscall implementations are derived from the C implementations in
`cloudabi-utils`. See `LICENSE.cloudabi-utils` for license information.
