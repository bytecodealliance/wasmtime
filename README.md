# Wasmtime: a WebAssembly Runtime.

Wasmtime is a standalone wasm-only runtime for [WebAssembly], using the [Cranelift] JIT.

It runs WebAssembly code [outside of the Web], and can be used both as a command-line
utility or as a library embedded in a larger application.

[WebAssembly]: https://webassembly.org/
[Cranelift]: https://github.com/CraneStation/cranelift
[outside of the Web]: https://webassembly.org/docs/non-web/

[![Build Status](https://dev.azure.com/CraneStation/Wasmtime/_apis/build/status/CraneStation.wasmtime?branchName=master)](https://dev.azure.com/CraneStation/Wasmtime/_build/latest?definitionId=4&branchName=master)
[![Gitter chat](https://badges.gitter.im/CraneStation/CraneStation.svg)](https://gitter.im/CraneStation/Lobby)
![Minimum rustc 1.37](https://img.shields.io/badge/rustc-1.37+-green.svg)

Wasmtime passes the WebAssembly spec testsuite, and supports a new system
API proposal called [WebAssembly System Interface], or WASI.

Wasmtime includes a git submodule; in order to build it, it's necessary to
obtain a full checkout, like this:
```
git clone --recurse-submodules https://github.com/CraneStation/wasmtime.git
```

To build an optimized version of Wasmtime, use Cargo:

```
cargo build --release
```

There are Rust, C, and C++ toolchains that can compile programs with WASI. See
the [WASI intro][WASI intro] for more information, and the [WASI tutorial][WASI tutorial]
for a tutorial on compiling and running programs using WASI and wasmtime, as
well as an overview of the filesystem sandboxing system.

Wasmtime does not yet implement Spectre mitigations, such as those being
pioneered [by](https://www.wasmjit.org/blog/spectre-mitigations-part-1.html)
[wasmjit](https://www.wasmjit.org/blog/spectre-mitigations-part-2.html),
however this is a subject of ongoing research.

[CloudABI]: https://cloudabi.org/
[WebAssembly System Interface]: docs/WASI-overview.md
[WASI intro]: docs/WASI-intro.md
[WASI tutorial]: docs/WASI-tutorial.md

Additional goals for Wasmtime include:
 - Support a variety of host APIs (not just WASI Core), with fast calling sequences,
   and develop proposals for additional API modules to be part of WASI.
   [Reference Sysroot](https://github.com/WebAssembly/reference-sysroot).
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
