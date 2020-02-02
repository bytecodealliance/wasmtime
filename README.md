# Wasmtime: a WebAssembly Runtime

**A [Bytecode Alliance][BA] project**

Wasmtime is a standalone wasm-only optimizing runtime for [WebAssembly] and
[WASI]. It runs WebAssembly code [outside of the Web], and can be used both
as a command-line utility or as a library embedded in a larger application.

To get started, visit [wasmtime.dev](https://wasmtime.dev/).

[BA]: https://bytecodealliance.org/
[WebAssembly]: https://webassembly.org/
[WASI]: https://wasi.dev
[outside of the Web]: https://webassembly.org/docs/non-web/
[build-status]: https://github.com/bytecodealliance/wasmtime/workflows/CI/badge.svg
[github-actions]: https://github.com/bytecodealliance/wasmtime/actions?query=workflow%3ACI
[gitter-chat-badge]: https://badges.gitter.im/CraneStation/CraneStation.svg
[gitter-chat]: https://gitter.im/CraneStation/Lobby
[minimum-rustc]: https://img.shields.io/badge/rustc-1.37+-green.svg

[![build-status]][github-actions]
[![gitter-chat-badge]][gitter-chat]
![minimum-rustc]

There are Rust, C, and C++ toolchains that can compile programs with WASI. See
the [WASI intro][WASI intro] for more information, and the [WASI tutorial][WASI tutorial]
for a tutorial on compiling and running programs using WASI and wasmtime, as
well as an overview of the filesystem sandboxing system.

Wasmtime passes the [WebAssembly spec testsuite]. To run it, update the
`tests/spec_testsuite` submodule with `git submodule update --remote`, and it
will be run as part of `cargo test`.

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
 - Facilitate development and testing around the [Cranelift] and [Lightbeam] JITs,
   and other WebAssembly execution strategies.
 - Develop a native ABI used for compiling WebAssembly suitable for use in both
   JIT and AOT to native object files.

[Cranelift]: https://github.com/bytecodealliance/cranelift
[Lightbeam]: https://github.com/bytecodealliance/wasmtime/tree/master/crates/lightbeam

## Using Wasmtime

There are two primary ways to use Wasmtime: as a standalone runtime for running WASI-compatible WASM binaries, and via the embedding API in Rust and C.

#### Including Wasmtime in your project

Wasmtime exposes an API for embedding as a library through the `wasmtime` subcrate,
which contains both a [high-level and safe Rust API], as well as a C-compatible API
compatible with the [proposed WebAssembly C API].

For more information, see the [Rust API embedding chapter] of the Wasmtime documentation.

[high-level and safe Rust API]: https://docs.rs/wasmtime/
[proposed WebAssembly C API]: https://github.com/WebAssembly/wasm-c-api
[Rust API embedding chapter]: https://bytecodealliance.github.io/wasmtime/embed-rust.html

It's Wasmtime.

#### Standalone runtime

The [Releases](https://github.com/bytecodealliance/wasmtime/releases) page has pre-built binaries for each release, with binaries for Linux, macOS and Windows.

If you want to build the latest Wasmtime from scratch, you will need:

* An updated [Rust toolchain](https://www.rust-lang.org/install.html) installed
* A C toolchain installed - e.g. the [`build-essential`](https://packages.ubuntu.com/xenial/build-essential) package in Ubuntu
* A clone of the Wasmtime Git repo

Example:

```sh
git clone https://github.com/bytecodealliance/wasmtime.git
git submodule update --init --recursive
cargo build --release
```

This will create a binary at `/target/release/wasmtime`, which you can then invoke directly to run a given WASI-compatible WASM binary:

```sh
$ ./target/release/wasmtime YOUR-BINARY.wasm
```

See `wasmtime --help` for the full list of options.

### Licence

Apache 2.0 licensed. See the LICENSE file for details.
