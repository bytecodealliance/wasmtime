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
![Minimum rustc 1.30](https://img.shields.io/badge/rustc-1.30+-green.svg)

*This is a work in progress that is not currently functional, but under active development.*

One goal for this project is to implement [CloudABI](https://cloudabi.org/) using
WebAssembly as the code format, provide [CloudABI system calls] as WebAssembly
host imports, and then port the [Rust CloudABI package] and [CloudABI libc] to it
to support Rust, C, C++, and other toolchains.

CloudABI is a natural complement for WebAssembly, since WebAssembly provides
sandboxing for code but doesn't have any builtin I/O, and CloudABI provides
sandboxed I/O.

[CloudABI]: https://cloudabi.org/
[CloudABI system calls]: https://github.com/NuxiNL/cloudabi#specification-of-the-abi
[Rust CloudABI package]: https://crates.io/crates/cloudabi
[CloudABI libc]: https://github.com/NuxiNL/cloudlibc

Additional goals for Wasmtime include:
 - Support a variety of host APIs (not just CloudABI), with fast calling sequences,
   and develop proposals for system calls in the WebAssembly
   [Reference Sysroot](https://github.com/WebAssembly/reference-sysroot).
 - Implement the [proposed WebAssembly C API].
 - Facilitate testing, experimentation, and development around the [Cranelift] and
   [Lightbeam] JITs.
 - Develop a the native ABI used for compiling WebAssembly suitable for use in both
   JIT and AOT to native object files.

[proposed WebAssembly C API]: https://github.com/rossberg/wasm-c-api
[Cranelift]: https://github.com/CraneStation/cranelift
[Lightbeam]: https://github.com/CraneStation/lightbeam

It's Wasmtime.
