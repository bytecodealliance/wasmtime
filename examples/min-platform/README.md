# Example: Minimal Platform Build of Wasmtime

This example is a showcase of what it looks like to build Wasmtime with a
minimal set of platform dependencies. This might be suitable when running
WebAssembly outside of Linux on a smaller system with a custom operating system
for example. Support here is built on Wasmtime's support of "custom platforms"
and more details can be found [online as
well](https://docs.wasmtime.dev/examples-minimal.html).

The example is organized into a few locations:

* `examples/min-platform/embedding/{Cargo.toml,src}` - source code for the
  embedding of Wasmtime itself. This is compiled to the target architecture
  and will have a minimal set of dependencies.

* `examples/min-platform/embedding/wasmtime-platform.{h,c}` - an example
  implementation of the platform dependencies that Wasmtime requires. This is
  defined and documented in
  `crates/wasmtime/src/runtime/vm/sys/custom/capi.rs`. The example here
  implements the required functions with Linux syscalls. Note that by default
  most of the file is not necessary to implement and is gated by
  `WASMTIME_VIRTUAL_MEMORY` and `WASMTIME_NATIVE_SIGNALS`. These correspond
  to the `custom-virtual-memory` and `custom-native-signals` crate features of
  `wasmtime` which are off-by-default and are optional performance
  optimizations.

* `examples/min-platform/{Cargo.toml,src}` - an example "host embedding" which
  loads and runs the `embedding` from above. This is a bit contrived and mostly
  serves as a bit of a test case for Wasmtime itself to execute in CI. The
  general idea though is that this is a Linux program which will load the
  `embedding` project above and execute it to showcase that the code works.

* `examples/min-platform/build.sh` - a script to build/run this example.

Taken together this example is unlikely to satisfy any one individual use case
but should set up the scaffolding to show how Wasmtime can be built for a
nonstandard platform. Wasmtime effectively requires one pointer of thread-local
memory and otherwise all other dependencies can be internalized.

## Description

This example will compile Wasmtime to any Rust target specified. The embedding
will run a few small examples of WebAssembly modules and then return. This
example is built in Wasmtime's CI with `x86_64-unknown-none` for example as a
Rust target.

The host for this is a Linux program which supplies the platform dependencies
that the embedding requires, for example the `wasmtime_*` symbols. This host
program will load the embedding and execute it. This is mostly specific to
executing this example in CI and is not necessarily representative of a "real"
embedding where you'd probably use static linking instead of dynamic linking
for example at the very least.

## Running this example

This example can be built and run with the `./build.sh` script in this
directory.
