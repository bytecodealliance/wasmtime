# wasi-common
[![travis-build-status]][travis] [![appveyor-build-status]][appveyor] [![rustc-1.35]][rustc]

[travis-build-status]: https://travis-ci.org/CraneStation/wasi-common.svg?branch=master
[travis]: https://travis-ci.org/CraneStation/wasi-common
[appveyor-build-status]: https://ci.appveyor.com/api/projects/status/github/cranestation/wasi-common?svg=true
[appveyor]: https://ci.appveyor.com/project/cranestation/wasi-common
[rustc-1.35]: https://img.shields.io/badge/rustc-1.35+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2019/05/23/Rust-1.35.0.html
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

Please note that the library requires Rust compiler version at least 1.35.0.

## Supported syscalls

### *nix
In our *nix implementation, we currently support the entire [WASI API]
with the exception of socket hostcalls:
- `sock_recv`
- `sock_send`
- `sock_shutdown`

We expect these to be implemented when network access is standardised.

We also currently do not support the `proc_raise` hostcall, as it is expected to
be dropped entirely from WASI.

### Windows
In our Windows implementation, we currently support the minimal subset of [WASI API]
which allows for running the very basic "Hello world!" style WASM apps. More coming shortly,
so stay tuned!

## Third-Party Code
Significant parts of our hostcall implementations are derived from the C implementations in
`cloudabi-utils`. See [LICENSE.cloudabi-utils](LICENSE.cloudabi-utils) for license information.
