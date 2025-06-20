# Building

This section describes everything required to build and run Wasmtime.

## Prerequisites

Before we can actually build Wasmtime, we'll need to make sure these things are
installed first.

### Git Submodules

The Wasmtime repository contains a number of git submodules. To build Wasmtime
and most other crates in the repository, you have to ensure that those are
initialized with this command:

```console
git submodule update --init
```

### The Rust Toolchain

[Install the Rust toolchain here.](https://www.rust-lang.org/tools/install) This
includes `rustup`, `cargo`, `rustc`, etc...

### `libclang` (optional)

The `wasmtime-fuzzing` crate transitively depends on `bindgen`, which requires
that your system has a `libclang` installed. Therefore, if you want to hack on
Wasmtime's fuzzing infrastructure, you'll need `libclang`. [Details on how to
get `libclang` and make it available for `bindgen` are
here.](https://rust-lang.github.io/rust-bindgen/requirements.html#clang)

## Building the `wasmtime` CLI

To make an unoptimized, debug build of the `wasmtime` CLI tool, go to the root
of the repository and run this command:

```console
cargo build
```

The built executable will be located at `target/debug/wasmtime`.

To make an optimized build, run this command in the root of the repository:

```console
cargo build --release
```

The built executable will be located at `target/release/wasmtime`.

You can also build and run a local `wasmtime` CLI by replacing `cargo build`
with `cargo run`.

## Building the Wasmtime C API

See
[`crates/c-api/README.md`](https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/README.md)
for details.

## Building Other Wasmtime Crates

You can build any of the Wasmtime crates by appending `-p wasmtime-whatever` to
the `cargo build` invocation. For example, to build the `wasmtime-environ` crate,
execute this command:

```console
cargo build -p wasmtime-environ
```

Alternatively, you can `cd` into the crate's directory, and run `cargo build`
there, without needing to supply the `-p` flag:

```console
cd crates/environ/
cargo build
```
