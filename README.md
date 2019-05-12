# wasi-common
[![build-status]][travis] [![rustc-1.34]][rustc]

[build-status]: https://travis-ci.org/CraneStation/wasi-common.svg?branch=master
[travis]: https://travis-ci.org/CraneStation/wasi-common
[rustc-1.34]: https://img.shields.io/badge/rustc-1.34+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2019/04/11/Rust-1.34.0.html
[Wasmtime]: https://github.com/CraneStation/wasmtime
[Lucet]: https://github.com/fastly/lucet
[lucet-wasi]: https://github.com/fastly/lucet/tree/master/lucet-wasi
[lucet-wasi-tracker]: https://github.com/fastly/lucet/commit/5d3efb6005391a7c71d585732a5507b00db6bb1e
[WASI API]: https://github.com/CraneStation/wasmtime/blob/master/docs/WASI-api.md

This repo will ultimately serve as a library providing a common implementation of
WASI hostcalls for re-use in any WASI (and potentially non-WASI) runtimes
such as [Wasmtime] and [Lucet].

The library is an adaption of [lucet-wasi] crate from the [Lucet] project, and it is
currently based on [5d3efb6005][lucet-wasi-tracker] git revision.

Please note that the library requires Rust compiler version at least 1.34.0.

## Supported syscalls

We support a subset of the [WASI API], though we are working on new hostcalls
on a regular basis. We currently implement:

- `__wasi_args_get`
- `__wasi_args_sizes_get`
- `__wasi_clock_res_get`
- `__wasi_clock_time_get`
- `__wasi_environ_get`
- `__wasi_environ_sizes_get`
- `__wasi_fd_close`
- `__wasi_fd_fdstat_get`
- `__wasi_fd_fdstat_set_flags`
- `__wasi_fd_filestat_get`
- `__wasi_fd_prestat_dir_name`
- `__wasi_fd_prestat_get`
- `__wasi_fd_read`
- `__wasi_fd_seek`
- `__wasi_fd_write`
- `__wasi_path_open`
- `__wasi_path_filestat_get`
- `__wasi_path_create_directory`
- `__wasi_path_unlink_file`
- `__wasi_poll_oneoff`
- `__wasi_proc_exit`
- `__wasi_random_get`

This is enough to run basic C and Rust programs, including those that use command-line arguments,
environment variables, stdio, and basic file operations.

## Third-Party Code
Significant parts of our hostcall implementations are derived from the C implementations in
`cloudabi-utils`. See [LICENSE.cloudabi-utils](LICENSE.cloudabi-utils) for license information.
