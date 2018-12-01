# Wasmtime: a WebAssembly Runtime.

A standalone wasm-only runtime for [WebAssembly], using [Cranelift].

[![Travis Status](https://travis-ci.org/CraneStation/wasmtime.svg?branch=master)](https://travis-ci.org/CraneStation/wasmtime)
[![Appveyor Status](https://ci.appveyor.com/api/projects/status/vxvpt2plriy5s0mc?svg=true)](https://ci.appveyor.com/project/CraneStation/cranelift)
[![Gitter chat](https://badges.gitter.im/CraneStation/CraneStation.svg)](https://gitter.im/CraneStation/Lobby)
![Minimum rustc 1.30](https://img.shields.io/badge/rustc-1.30+-green.svg)

*This is a work in progress that is not currently functional, but under active development.*

Goals include:
 - Be a general-purpose engine for running WebAssembly code [outside of browsers].
 - Support a variety of host APIs with fast calling sequences.
 - Prototype syscall APIs that can be proposed for use in the WebAssembly
   [Reference Sysroot](https://github.com/WebAssembly/reference-sysroot).
 - Facilitate testing, experimentation, and development around the [Cranelift] and
   [Lightbeam] JITs.
 - Develop a the native ABI used for compiling WebAssembly suitable for use in
   both JIT and AOT to native object files.

[WebAssembly]: https://webassembly.org/
[outside of browsers]: https://github.com/WebAssembly/design/blob/master/NonWeb.md
[Cranelift]: https://github.com/CraneStation/cranelift
[Lightbeam]: https://github.com/CraneStation/lightbeam

It's Wasmtime.
