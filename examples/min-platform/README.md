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

* `examples/min-platform/embedding/*.json` - custom Rust target definitions
  which are used when compiling this example. These are the custom target files
  that are the compilation target of the `embedding` crate. This is a feature
  of nightly Rust to be able to use these. Note that the contents can be
  customized and these files are only examples.

* `examples/min-platform/embedding/wasmtime-platform.{h,c}` - an example
  implementation of the platform dependencies that Wasmtime requires. This is
  defined and documented in
  `crates/wasmtime/src/runtime/vm/sys/custom/capi.rs`. The example here
  implements the required functions with Linux syscalls.

* `examples/min-platform/{Cargo.toml,src}` - an example "host embedding" which
  loads and runs the `embedding` from above. This is a bit contrived and mostly
  serves as a bit of a test case for Wasmtime itself to execute in CI. The
  general idea though is that this is a Linux program which will load the
  `embedding` project above and execute it to showcase that the code works.

* `examples/min-platform/build.sh` - a script to build/run this example.

Taken together this example is unlikely to satisfy any one individual use case
but should set up the scaffolding to show how Wasmtime can be built for a
nonstandard platform. Wasmtime effectively only has one requirement from the
system which is management of virtual memory, and beyond that everything else
can be internalized.

Note that at this time this support all relies on the fact that the Rust
standard library can be built for a custom target. Most of the Rust standard
library will be "stubbed out" however and won't work (e.g. opening a file would
return an error). This means that not all of the `wasmtime` crate will work, nor
will all features of the `wasmtime` crate, but the set of features activated
here should suffice.

## Description

This example will compile Wasmtime to a custom Rust target specified in
`*.json` files. This custom target, for the example, is modeled after Linux
except for the fact that Rust won't be able to know that (e.g. the `#[cfg]`
directives aren't set so code won't know it actually runs on Linux). The
embedding will run a few small examples of WebAssembly modules and then return.

The host for this is a Linux program which supplies the platform dependencies
that the embedding requires, for example the `wasmtime_*` symbols. This host
program will load the embedding and execute it.

## Points of Note

* Due to the usage of custom `*.json` targets, this example requires a nightly
  Rust compiler.
* Compiling the embedding requires `--cfg wasmtime_custom_platform` in the
  `RUSTFLAGS` environment variable. to indicate that Wasmtime's custom C
  API-based definition of platform support is desired.
* Due to the usage of a custom target most of libstd doesn't work. For example
  panics can't print anything and the process can only abort.
* Due to the custom target not all features of Wasmtime can be enabled because
  some crates may require platform functionality which can't be defined due to
  the lack of knowledge of what platform is being targeted.

## Running this example

This example can be built and run with the `./build.sh` script in this
directory. Example output looks like.
