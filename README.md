# wasi-common
[![build-status]][travis] [![rustc-1.34]][rustc]

[build-status]: https://travis-ci.org/CraneStation/wasi-common.svg?branch=master
[travis]: https://travis-ci.org/CraneStation/wasi-common
[rustc-1.34]: https://img.shields.io/badge/rustc-1.34+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2019/04/11/Rust-1.34.0.html
[Wasmtime]: https://github.com/CraneStation/wasmtime
[Lucet]: https://github.com/fastly/lucet
[lucet-wasi]: https://github.com/fastly/lucet/tree/master/lucet-wasi
[lucet-wasi-tracker]: https://github.com/fastly/lucet/commit/40ae1df64536250a2b6ab67e7f167d22f4aa7f94
[WASI API]: https://github.com/CraneStation/wasmtime/blob/master/docs/WASI-api.md

This repo will ultimately serve as a library providing a common implementation of
WASI hostcalls for re-use in any WASI (and potentially non-WASI) runtimes
such as [Wasmtime] and [Lucet].

The library is an adaption of [lucet-wasi] crate from the [Lucet] project, and it is
currently based on [40ae1df][lucet-wasi-tracker] git revision.

Please note that the library requires Rust compiler version at least 1.34.0.

## Supported syscalls

We support a subset of the [WASI API], though we are working on new hostcalls
on a regular basis. We currently implement:

- `args_get`
- `args_sizes_get`
- `clock_res_get`
- `clock_time_get`
- `environ_get`
- `environ_sizes_get`
- `fd_close`
- `fd_datasync`
- `fd_pread`
- `fd_pwrite`
- `fd_read`
- `fd_renumber`
- `fd_seek`
- `fd_tell`
- `fd_fdstat_get`
- `fd_fdstat_set_flags`
- `fd_fdstat_set_rights`
- `fd_sync`
- `fd_write`
- `fd_advise`
- `fd_allocate`
- `path_create_directory`
- `path_link`
- `path_open`
- `fd_readdir`
- `path_readlink`
- `path_rename`
- `fd_filestat_get`
- `fd_filestat_set_times`
- `fd_filestat_set_size`
- `path_filestat_get`
- `path_filestat_set_times`
- `path_symlink`
- `path_unlink_file`
- `path_remove_directory`
- `poll_oneoff`
- `fd_prestat_get`
- `fd_prestat_dir_name`
- `proc_exit`
- `random_get`
- `sched_yield`

This is enough to run basic C and Rust programs, including those that use command-line arguments,
environment variables, stdio, and basic file operations.

## Third-Party Code
Significant parts of our hostcall implementations are derived from the C implementations in
`cloudabi-utils`. See [LICENSE.cloudabi-utils](LICENSE.cloudabi-utils) for license information.
