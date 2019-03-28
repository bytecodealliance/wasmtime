# Wasmtime: a WebAssembly Runtime.

Wasmtime is a standalone wasm-only runtime for [WebAssembly], using the [Cranelift] JIT.

It runs WebAssembly code [outside of the Web], and can be used both as a command-line
utility or as a library embedded in a larger application.

[WebAssembly]: https://webassembly.org/
[Cranelift]: https://github.com/CraneStation/cranelift
[outside of the Web]: https://webassembly.org/docs/non-web/

[![Travis Status](https://travis-ci.org/CraneStation/wasmtime.svg?branch=master)](https://travis-ci.org/CraneStation/wasmtime)
[![Appveyor Status](https://ci.appveyor.com/api/projects/status/vxvpt2plriy5s0mc?svg=true)](https://ci.appveyor.com/project/CraneStation/cranelift)
[![Gitter chat](https://badges.gitter.im/CraneStation/CraneStation.svg)](https://gitter.im/CraneStation/Lobby)
![Minimum rustc 1.32](https://img.shields.io/badge/rustc-1.32+-green.svg)

Wasmtime passes the WebAssembly spec testsuite, and supports a new system
API proposal called [WebAssembly System Interface], or WASI.

There are Rust, C, and C++ toolchains that can compile programs with WASI. See
[here][WASI intro] for more information, and [here][WASI tutorial] for a
tutorial on compiling and running programs using WASI and wasmtime, as
well as an overview of the filesystem sandboxing system.

Wasmtime does not yet implement Spectre mitiations, such as those being
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

It's Wasmtime.
