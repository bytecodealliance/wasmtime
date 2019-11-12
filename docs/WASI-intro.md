# Welcome to WASI!

WASI stands for WebAssembly System Interface. It's an API designed by
the [Wasmtime] project that provides access to several operating-system-like
features, including files and filesystems, Berkeley sockets, clocks, and
random numbers, that we'll be proposing for standardization.

It's designed to be independent of browsers, so it doesn't depend on
Web APIs or JS, and isn't limited by the need to be compatible with JS.
And it has integrated capability-based security, so it extends
WebAssembly's characteristic sandboxing to include I/O.

See the [WASI Overview](WASI-overview.md) for more detailed background
information, and the [WASI Tutorial](WASI-tutorial.md) for a walkthrough
showing how various pieces fit together.

Note that everything here is a prototype, and while a lot of stuff works,
there are numerous missing features and some rough edges. For example,
networking support is incomplete.

## How can I write programs that use WASI?

The two toolchains that currently work well are the Rust toolchain and
a specially packaged C and C++ toolchain. Of course, we hope other
toolchains will be able to implement WASI as well!

### Rust

To install a WASI-enabled Rust toolchain:

```
rustup target add wasm32-wasi
cargo build --target wasm32-wasi
```

Until now, Rust's WebAssembly support has had two main options, the
Emscripten-based option, and the wasm32-unknown-unknown option. The latter
option is lighter-weight, but only supports `no_std`. WASI enables a new
wasm32-wasi target, which is similar to wasm32-unknown-unknown in
that it doesn't depend on Emscripten, but it can use WASI to provide a
decent subset of libstd.

### C/C++

All the parts needed to support wasm are included in upstream clang, lld, and
compiler-rt, as of the LLVM 8.0 release. However, to use it, you'll need
to build WebAssembly-targeted versions of the library parts, and it can
be tricky to get all the CMake invocations lined up properly.

To make things easier, we provide
[prebuilt packages](https://github.com/CraneStation/wasi-sdk/releases)
that provide builds of Clang and sysroot libraries.

WASI doesn't yet support `setjmp`/`longjmp` or C++ exceptions, as it is
waiting for [unwinding support in WebAssembly].

[unwinding support in WebAssembly]: https://github.com/WebAssembly/exception-handling/

Some C++ programs, particularly those using `<iostream>`, may see warnings
about function signature mismatches; this is a
[known bug](https://bugs.llvm.org/show_bug.cgi?id=40412).

## How can I run programs that use WASI?

Currently the options are [Wasmtime] and the [browser polyfill], though we
intend WASI to be implementable in many wasm VMs.

[Wasmtime]: https://github.com/bytecodealliance/wasmtime
[browser polyfill]: https://wasi.dev/polyfill/

### Wasmtime

[Wasmtime] is a non-Web WebAssembly engine which is part of the
[CraneStation project](https://github.com/CraneStation/). To build
it, download the code and build with `cargo build --release`. It can
run WASI-using wasm programs by simply running `wasmtime foo.wasm`,
or `cargo run --bin wasmtime foo.wasm`.

### The browser polyfill

The polyfill is online [here](https://wasi.dev/polyfill/).

The source is [here](https://github.com/bytecodealliance/wasmtime/tree/master/wasmtime-wasi/js-polyfill).

## Where can I learn more?

Beyond the [WASI Overview](WASI-overview.md), take a look at the
various [WASI documents](WASI-documents.md).
