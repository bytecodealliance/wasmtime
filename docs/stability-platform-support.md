# Platform Support

The `wasmtime` project is a configurable and lightweight runtime for WebAssembly
which has a number of ways it can be configured. Not all features are supported
on all platforms, but it is intended that `wasmtime` can run in some capacity on
almost all platforms! The matrix of what's being tested, what works, and what's
supported where is evolving over time, and this document hopes to capture a
snapshot of what the current state of the world looks like.

All features of `wasmtime` should work on the following platforms:

* Linux x86\_64
* macOS x86\_64
* Windows x86\_64

For more detailed information about supported platforms, please check out the
sections below!

## JIT compiler support

The JIT compiler, backed by either `lightbeam` or `cranelift` supports only the
x86\_64 architecture at this time. Support for at least ARM, AArch64, and x86 is
planned at this time.

Usage of the JIT compiler will require a host operating system which supports
creating executable memory pages on-the-fly. In Rust terms this generally means
that `std` needs to be supported on this platform.

## Interpreter support

At this time `wasmtime` does not have a mode in which it simply interprets
WebAssembly code. It is planned to add support for an interpreter, however, and
this will have minimal system dependencies. It is planned that the system will
need to support some form of dynamic memory allocation, but other than that not
much else will be needed.

## What about `#[no_std]`?

The `wasmtime` project does not currently use `#[no_std]` for its crates, but
this is not because it won't support it! At this time we're still gathering use
cases for for what `#[no_std]` might entail, so if you're interested in this
we'd love to hear about your use case! Feel free to open an issue on the
`wasmtime` repository to discuss this.
