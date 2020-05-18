# Building

This section describes everything required to build and run Wasmtime.

## Prerequisites

Before we can actually build Wasmtime, we'll need to make sure these things are
installed first.

### Git Submodules

The Wasmtime repository contains a number of git submodules. To build Wasmtime
and most other crates in the repository, you have to ensure that those are
initialized with this command:

```shell
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

```shell
cargo build
```

The built executable will be located at `target/debug/wasmtime`.

To make an optimized build, run this command in the root of the repository:

```shell
cargo build --release
```

The built executable will be located at `target/release/wasmtime`.

You can also build and run a local `wasmtime` CLI by replacing `cargo build`
with `cargo run`.

## Building the Wasmtime C API

To build the C API of Wasmtime you can run:

```shell
cargo build --release --manifest-path crates/c-api/Cargo.toml
```

This will place the shared library inside of `target/release`. On Linux it will
be called `libwasmtime.{a,so}`, on macOS it will be called
`libwasmtime.{a,dylib}`, and on Windows it will be called
`wasmtime.{lib,dll,dll.lib}`.

## Building Other Wasmtime Crates

You can build any of the Wasmtime crates by appending `-p wasmtime-whatever` to
the `cargo build` invocation. For example, to build the `wasmtime-jit` crate,
execute this command:

```shell
cargo build -p wasmtime-jit
```

Alternatively, you can `cd` into the crate's directory, and run `cargo build`
there, without needing to supply the `-p` flag:

```shell
cd crates/jit/
cargo build
```

## Cross Compiling Wasmtime

By default `cargo build` will build Wasmtime for the platform you're running the
build on. You might, however, want to build Wasmtime for a different platform!
Let's say for example that you want to build Wasmtime for
`aarch64-unknown-linux-gnu`. First you'll want to acquire the Rust standard
library for this target:

```shell
rustup target add aarch64-unknown-linux-gnu
```

Next you need to install a native C toolchain which has a C compiler, runtime
libraries, and linker for the desired target. This is unfortunately not very
easy to acquire on most platforms:

* On Windows you can install build tools for AArch64 Windows, but targeting
  platforms like Linux or macOS is not easy. While toolchains exist for
  targeting non-Windows platforms you'll have to hunt yourself to find the right
  one.

* On macOS you can install, through Xcode, toolchains for iOS but the main
  `x86_64-apple-darwin` is really the only easy target to install. You'll need
  to hunt for toolchains if you want to compile for Linux or Windows.

* On Linux you can relatively easily compile for other Linux architectures most
  of the time. For example on Debian-based distributions you can install the
  `gcc-aarch64-linux-gnu` package which should come with the C compiler, runtime
  libraries, and linker all in one (assuming you don't explicitly request
  disabling recommended packages). Other Linux distributions may have
  differently named toolchains. Compiling for macOS from Linux will require
  finding your own toolchain. Compiling for Windows MSVC will require finding
  your own toolchain, but compiling for MinGW can work easily enough if you
  install the MinGW toolchain via your package manager.

For now we'll assume you're on Linux compiling for a different Linux
architecture.  Once you've got the native toolchain, you'll want to find the C
compiler that came with it. On Debian, for example, this is called
`aarch64-linux-gnu-gcc`. Next up you'll need to configure two environment
variables to configure the Rust build:

```shell
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
```

The first environment variable tells Cargo to tell rustc what the correct linker
for your target is. The second configures the [`cc` Rust
crate](https://crates.io/crates/cc) for C code compiled as part of the build.

Finally you can execute.

```shell
cargo build --target aarch64-unknown-linux-gnu --release
```

The built executable will be located at
`target/aarch64-unknown-linux-gnu/release/wasmtime`. Note that you can
cross-compile the C API in the same manner as the CLI too.
