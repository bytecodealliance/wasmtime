# Wasmtime: a WebAssembly Runtime.

Wasmtime is a standalone wasm-only optimizing runtime for [WebAssembly] and
[WASI]. It runs WebAssembly code [outside of the Web], and can be used both
as a command-line utility or as a library embedded in a larger application.

To get started, visit [wasmtime.dev](https://wasmtime.dev/).

[WebAssembly]: https://webassembly.org/
[WASI]: https://wasi.dev
[outside of the Web]: https://webassembly.org/docs/non-web/

[![Build Status](https://dev.azure.com/CraneStation/Wasmtime/_apis/build/status/CraneStation.wasmtime?branchName=master)](https://dev.azure.com/CraneStation/Wasmtime/_build/latest?definitionId=4&branchName=master)
[![Gitter chat](https://badges.gitter.im/CraneStation/CraneStation.svg)](https://gitter.im/CraneStation/Lobby)
![Minimum rustc 1.37](https://img.shields.io/badge/rustc-1.37+-green.svg)

There are Rust, C, and C++ toolchains that can compile programs with WASI. See
the [WASI intro][WASI intro] for more information, and the [WASI tutorial][WASI tutorial]
for a tutorial on compiling and running programs using WASI and wasmtime, as
well as an overview of the filesystem sandboxing system.

Wasmtime passes the [WebAssembly spec testsuite]. To run it, update the
`spec_testsuite` submodule with `git submodule update --remote`, and it will
be run as part of `cargo test`.

Wasmtime does not yet implement Spectre mitigations, however this is a subject
of ongoing research.

[WebAssembly spec testsuite]: https://github.com/WebAssembly/testsuite
[CloudABI]: https://cloudabi.org/
[WebAssembly System Interface]: docs/WASI-overview.md
[WASI intro]: docs/WASI-intro.md
[WASI tutorial]: docs/WASI-tutorial.md

Additional goals for Wasmtime include:
 - Support a variety of host APIs (not just WASI), with fast calling sequences,
   and develop proposals for additional API modules to be part of WASI.
 - Implement the [proposed WebAssembly C API].
 - Facilitate testing, experimentation, and development around the [Cranelift] and
   [Lightbeam] JITs.
 - Develop a native ABI used for compiling WebAssembly suitable for use in both
   JIT and AOT to native object files.

[proposed WebAssembly C API]: https://github.com/rossberg/wasm-c-api
[Cranelift]: https://github.com/CraneStation/cranelift
[Lightbeam]: https://github.com/CraneStation/lightbeam

#### Including Wasmtime in your project
Wasmtime exposes an API for JIT compilation through the `wasmtime-jit` subcrate, which depends on `wasmtime-environ` and `wasmtime-runtime` for the ABI and runtime support respectively. However, this API is not documented and subject to change. Please use at your own risk!

Build the individual crates as such:

```
cargo build --package wasmtime-jit
```

Wasmtime does not currently publish these crates on crates.io. They may be included as a git dependency, like this:

```toml
[dependencies]
wasmtime-environ = { git = "https://github.com/CraneStation/wasmtime", rev = "somecommithash" }
wasmtime-runtime = { git = "https://github.com/CraneStation/wasmtime", rev = "somecommithash" }
wasmtime-jit = { git = "https://github.com/CraneStation/wasmtime", rev = "somecommithash" }
```

All three crates must be specified as dependencies for `wasmtime-jit` to build correctly, at the moment.

It's Wasmtime.
