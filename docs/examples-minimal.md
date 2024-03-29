# Building a Minimal Wasmtime embedding

Wasmtime embeddings may wish to optimize for binary size and runtime footprint
to fit on a small system. This documentation is intended to guide some features
of Wasmtime and how to best produce a minimal build of Wasmtime.

## Building a minimal CLI

> *Note*: the exact numbers in this section were last updated on 2023-10-18 on a
> macOS aarch64 host. For up-to-date numbers consult the artifacts in the [`dev`
> release of Wasmtime][dev] where the `wasmtime-min` executable represents the
> culmination of these steps.

[dev]: https://github.com/bytecodealliance/wasmtime/releases/tag/dev

Many Wasmtime embeddings go through the `wasmtime` crate as opposed to the
`wasmtime` CLI executable, but to start out let's take a look at minimizing the
command line executable. By default the wasmtime command line executable is
relatively large:

```shell
$ cargo build
$ ls -l ./target/debug/wasmtime
-rwxr-xr-x@ 1 root  root    140M Oct 18 08:33 target/debug/wasmtime
```

The easiest size optimization is to compile with optimizations. This will strip
lots of dead code and additionally generate much less debug information by
default

```shell
$ cargo build --release
$ ls -l ./target/release/wasmtime
-rwxr-xr-x@ 1 root  root     33M Oct 18 08:34 target/release/wasmtime
```

Much better, but still relatively large! The next thing that can be done is to
disable the default features of the `wasmtime-cli` crate. This will remove all
optional functionality from the crate and strip it down to the bare bones
functionality. Note though that `run` is included to keep the ability to run
precompiled WebAssembly files as otherwise the CLI doesn't have any
functionality which isn't too useful.

```shell
$ cargo build --release --no-default-features --features run
$ ls -l ./target/release/wasmtime
-rwxr-xr-x@ 1 root  root    6.7M Oct 18 08:37 target/release/wasmtime
```

Note that this executable is stripped to the bare minimum of functionality which
notably means it does not have a compiler for WebAssembly files. This means that
`wasmtime compile` is no longer supported meaning that `*.cwasm` files must be
fed to `wasmtime run` to execute files. Additionally error messages will be
worse in this mode as less contextual information is provided.

The final Wasmtime-specific optimization you can apply is to disable logging
statements. Wasmtime and its dependencies make use of the [`log`
crate](https://docs.rs/log) and [`tracing` crate](https://docs.rs/tracing) for
debugging and diagnosing. For a minimal build this isn't needed though so this
can all be disabled through Cargo features to shave off a small amount of code.
Note that for custom embeddings you'd need to replicate the `disable-logging`
feature which sets the `max_level_off` feature for the `log` and `tracing`
crate.

```shell
$ cargo build --release --no-default-features --features run,disable-logging
$ ls -l ./target/release/wasmtime
-rwxr-xr-x@ 1 root  root    6.7M Oct 18 08:37 target/release/wasmtime
```

At this point the next line of tricks to apply to minimize binary size are
[general tricks-of-the-trade for Rust
programs](https://github.com/johnthagen/min-sized-rust) and are no longer
specific to Wasmtime. For example the first thing that can be done is to
optimize for size rather than speed via rustc's `s` optimization level.
This uses Cargo's [environment-variable based configuration][cargo-env-config]
via the `CARGO_PROFILE_RELEASE_OPT_LEVEL=s` environment variable to configure
this.

[cargo-env-config]: https://doc.rust-lang.org/cargo/reference/config.html#profile

```shell
$ export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
$ cargo build --release --no-default-features --features run,disable-logging
$ ls -l ./target/release/wasmtime
-rwxr-xr-x@ 1 root  root    6.8M Oct 18 08:40 target/release/wasmtime
```

Note that the size has increased here slightly instead of going down. Optimizing
for speed-vs-size can affect a number of heuristics in LLVM so it's best to test
out locally what's best for your embedding. Further examples below continue to
pass this flag since by the end it will produce a smaller binary than the
default optimization level of "3" for release mode. You may wish to also try an
optimization level of "2" and see which produces a smaller build for you.

After optimizations levels the next compilation setting to configure is
Rust's "panic=abort" mode where panics translate to process aborts rather than
unwinding. This removes landing pads from code as well as unwind tables from the
executable.

```shell
$ export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
$ export CARGO_PROFILE_RELEASE_PANIC=abort
$ cargo build --release --no-default-features --features run,disable-logging
$ ls -l ./target/release/wasmtime
-rwxr-xr-x@ 1 root  root    5.0M Oct 18 08:40 target/release/wasmtime
```

Next, if the compile time hit is acceptable, LTO can be enabled to provide
deeper opportunities for compiler optimizations to remove dead code and
deduplicate. Do note that this will take a significantly longer amount of time
to compile than previously. Here LTO is configured with
`CARGO_PROFILE_RELEASE_LTO=true`.

```shell
$ export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
$ export CARGO_PROFILE_RELEASE_PANIC=abort
$ export CARGO_PROFILE_RELEASE_LTO=true
$ cargo build --release --no-default-features --features run,disable-logging
$ ls -l ./target/release/wasmtime
-rwxr-xr-x@ 1 root  root    3.3M Oct 18 08:42 target/release/wasmtime
```

Similar to LTO above rustc can be further instructed to place all crates into
their own single object file instead of multiple by default. This again
increases compile times. Here that's done with
`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`.

```shell
$ export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
$ export CARGO_PROFILE_RELEASE_PANIC=abort
$ export CARGO_PROFILE_RELEASE_LTO=true
$ export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
$ cargo build --release --no-default-features --features run,disable-logging
$ ls -l ./target/release/wasmtime
-rwxr-xr-x@ 1 root  root    3.3M Oct 18 08:43 target/release/wasmtime
```

Note that with LTO using a single codegen unit may only have marginal benefit.
If not using LTO, however, a single codegen unit will likely provide benefit
over the default 16 codegen units.

One final flag before getting to nightly features is to strip debug information
from the standard library. In `--release` mode Cargo by default doesn't generate
debug information for local crates, but the Rust standard library may have debug
information still included with it. This is configured via
`CARGO_PROFILE_RELEASE_STRIP=debuginfo`

```shell
$ export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
$ export CARGO_PROFILE_RELEASE_PANIC=abort
$ export CARGO_PROFILE_RELEASE_LTO=true
$ export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
$ export CARGO_PROFILE_RELEASE_STRIP=debuginfo
$ cargo build --release --no-default-features --features run,disable-logging
$ ls -l ./target/release/wasmtime
-rwxr-xr-x@ 1 root  root    2.4M Oct 18 08:44 target/release/wasmtime
```

Next, if your use case allows it, the Nightly Rust toolchain provides a number
of other options to minimize the size of binaries. Note the usage of `+nightly` here
to the `cargo` command to use a Nightly toolchain (assuming your local toolchain
is installed with rustup). Also note that due to the nature of nightly the exact
flags here may not work in the future. Please open an issue with Wasmtime if
these commands don't work and we'll update the documentation.

The first nightly feature we can leverage is to remove filename and line number
information in panics with `-Zlocation-detail=none`

```shell
$ export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
$ export CARGO_PROFILE_RELEASE_PANIC=abort
$ export CARGO_PROFILE_RELEASE_LTO=true
$ export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
$ export CARGO_PROFILE_RELEASE_STRIP=debuginfo
$ export RUSTFLAGS="-Zlocation-detail=none"
$ cargo +nightly build --release --no-default-features --features run,disable-logging
$ ls -l ./target/release/wasmtime
-rwxr-xr-x@ 1 root  root    2.4M Oct 18 08:43 target/release/wasmtime
```

Further along the line of nightly features the next optimization will recompile
the standard library without unwinding information, trimming out a bit more from
the standard library. This uses the `-Zbuild-std` flag to Cargo. Note that this
additionally requires `--target` as well which will need to be configured for
your particular platform.

```shell
$ export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
$ export CARGO_PROFILE_RELEASE_PANIC=abort
$ export CARGO_PROFILE_RELEASE_LTO=true
$ export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
$ export CARGO_PROFILE_RELEASE_STRIP=debuginfo
$ export RUSTFLAGS="-Zlocation-detail=none"
$ cargo +nightly build --release --no-default-features --features run,disable-logging \
    -Z build-std=std,panic_abort --target aarch64-apple-darwin
$ ls -l ./target/aarch64-apple-darwin/release/wasmtime
-rwxr-xr-x@ 1 root  root    2.3M Oct 18 09:39 target/aarch64-apple-darwin/release/wasmtime
```

Next the Rust standard library has some optional features in addition to
Wasmtime, such as printing of backtraces. This may not be required in minimal
environments so the features of the standard library can be disabled with the
`-Zbuild-std-features=` flag which configures the set of enabled features to be
empty.

```shell
$ export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
$ export CARGO_PROFILE_RELEASE_PANIC=abort
$ export CARGO_PROFILE_RELEASE_LTO=true
$ export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
$ export CARGO_PROFILE_RELEASE_STRIP=debuginfo
$ export RUSTFLAGS="-Zlocation-detail=none"
$ cargo +nightly build --release --no-default-features --features run,disable-logging \
    -Z build-std=std,panic_abort --target aarch64-apple-darwin \
    -Z build-std-features=
$ ls -l ./target/aarch64-apple-darwin/release/wasmtime
-rwxr-xr-x@ 1 root  root    2.1M Oct 18 09:39 target/aarch64-apple-darwin/release/wasmtime
```

## Minimizing further

Above shows an example of taking the default `cargo build` result of 130M down
to a 2.1M binary for the `wasmtime` executable. Similar steps can be done to
reduce the size of the C API binary artifact as well which currently produces a
~2.8M dynamic library. This is currently the smallest size with the source code
as-is, but there are more size reductions which haven't been implemented yet.

This is a listing of some example sources of binary size. Some sources of binary
size may not apply to custom embeddings since, for example, your custom
embedding might already not use WASI and might already not be included.

* WASI in the Wasmtime CLI - currently the CLI includes all of WASI. This
  includes two separate implementations of WASI - one for preview2 and one for
  preview1. This accounts for 1M+ of space which is a significant chunk of the
  remaining 2.1M.  While removing just preview2 or preview1 would be easy enough
  with a Cargo feature, the resulting executable wouldn't be able to do
  anything. Something like a [plugin feature for the
  CLI](https://github.com/bytecodealliance/wasmtime/issues/7348), however, would
  enable removing WASI while still being a usable executable.

* Argument parsing in the Wasmtime CLI - as a command line executable `wasmtime`
  contains parsing of command line arguments which currently uses the `clap`
  crate. This contributes ~200k of binary size to the final executable which
  would likely not be present in a custom embedding of Wasmtime. While this
  can't be removed from Wasmtime it's something to consider when evaluating the
  size of CI artifacts.

* Cranelift in the C API - one of the features of Wasmtime is the ability to
  have a runtime without Cranelift that only supports precompiled (AOT) wasm
  modules. It's [not possible to build the C API without
  Cranelift](https://github.com/bytecodealliance/wasmtime/issues/7349) though
  because defining host functions requires Cranelift at this time to emit some
  stubs.  This means that the C API is significantly larger than a custom Rust
  embedding which doesn't suffer from the same restriction. This means that
  while it's still possible to build an embedding of Wasmtime which doesn't have
  Cranelift it's not easy to see what it might look like size-wise from
  looking at the C API artifacts.

* Formatting strings in Wasmtime - Wasmtime makes extensive use of formatting
  strings for error messages and other purposes throughout the implementation.
  Most of this is intended for debugging and understanding more when something
  goes wrong, but much of this is not necessary for a truly minimal embedding.
  In theory much of this could be conditionally compiled out of the Wasmtime
  project to produce a smaller executable. Just how much of the final binary
  size is accounted for by formatting string is unknown, but it's well known in
  Rust that `std::fmt` is not the slimmest of modules.

* Cranelift vs Winch - the "min" builds on CI try to exclude Cranelift from
  their binary footprint (e.g. the CLI excludes it) but this comes at a cost of
  the final executable not supporting compilation of wasm modules. If this is
  required then no effort has yet been put into minimizing the code size of
  Cranelift itself. One possible tradeoff that can be made though is to choose
  between the Winch baseline compiler vs Cranelift. Winch should be much smaller
  from a compiled footprint point of view while not sacrificing everything in
  terms of performance. Note though that Winch is still under development.

Above are some future avenues to take in terms of reducing the binary size of
Wasmtime and various tradeoffs that can be made. The Wasmtime project is eager
to hear embedder use cases/profiles if Wasmtime is not suitable for binary size
reasons today. Please feel free to [open an
issue](https://github.com/bytecodealliance/wasmtime/issues/new) and let us know
and we'd be happy to discuss more how best to handle a particular use case.

# Building Wasmtime for a Custom Platform

If you're not running on a built-in supported platform such as Windows, macOS,
or Linux, then Wasmtime won't work out-of-the-box for you. Wasmtime includes a
compilation mode, however, that enables you to define how to work with the
platform externally.

This mode is enabled when `--cfg wasmtime_custom_platform` is passed to rustc,
via `RUSTFLAGS` for example when building through Cargo, when an existing
platform is not matched. This means that with this configuration Wasmtime may be
compiled for custom or previously unknown targets.

Wasmtime's current "platform embedding API" which is required to operate is
defined at `examples/min-platform/embedding/wasmtime-platform.h`. That directory
additionally has an example of building a minimal `*.so` on Linux which has the
platform API implemented in C using Linux syscalls. While a bit contrived it
effectively shows a minimal Wasmtime embedding which has no dependencies other
than the platform API.

Building Wasmtime for a custom platform is not a turnkey process right now,
there are a number of points that need to be considered:

* For a truly custom platform you'll probably want to create a [custom Rust
  target](https://docs.rust-embedded.org/embedonomicon/custom-target.html). This
  means that Nightly Rust will be required.

* Wasmtime and its dependencies require the Rust standard library `std` to be
  available. The Rust standard library can be compiled for any target with
  unsupported functionality being stubbed out. This mode of compiling the Rust
  standard library is not stable, however. Currently this is done through the
  `-Zbuild-std` argument to Cargo along with a
  `+RUSTC_BOOTSTRAP_SYNTHETIC_TARGET=1` environment variable.

* Wasmtime additionally depends on the availability of a memory allocator (e.g.
  `malloc`). Wasmtime assumes that failed memory allocation aborts the process.

* Not all features for Wasmtime can be built for custom targets. For example
  WASI support does not work on custom targets. When building Wasmtime you'll
  probably want `--no-default-features` and will then want to incrementally add
  features back in as needed.

The `examples/min-platform` directory has an example of building this minimal
embedding and some necessary steps. Combined with the above features about
producing a minimal build currently produces a 400K library on Linux.
