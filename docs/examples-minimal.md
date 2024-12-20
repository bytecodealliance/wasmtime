# Building a Minimal Wasmtime embedding

Wasmtime embeddings may wish to optimize for binary size and runtime footprint
to fit on a small system. This documentation is intended to guide some features
of Wasmtime and how to best produce a minimal build of Wasmtime.

## Building a minimal CLI

> *Note*: the exact numbers in this section were last updated on 2024-12-12 on a
> Linux x86\_64 host. For up-to-date numbers consult the artifacts in the [`dev`
> release of Wasmtime][dev] where the `min/lib/libwasmtime.so` binary
> represents the culmination of these steps.

[dev]: https://github.com/bytecodealliance/wasmtime/releases/tag/dev

Many Wasmtime embeddings go through the `wasmtime` crate as opposed to the
Wasmtime C API `libwasmtime.so`, but to start out let's take a look at
minimizing the dynamic library as a case study. By default the C API is
relatively large:

```shell
$ cargo build -p wasmtime-c-api
$ ls -lh ./target/debug/libwasmtime.so
-rwxrwxr-x 2 alex alex 260M Dec 12 07:46 target/debug/libwasmtime.so
```

The easiest size optimization is to compile with optimizations. This will strip
lots of dead code and additionally generate much less debug information by
default

```shell
$ cargo build -p wasmtime-c-api --release
$ ls -lh ./target/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 19M Dec 12 07:46 target/release/libwasmtime.so
```

Much better, but still relatively large! The next thing that can be done is to
disable the default features of the C API. This will remove all
optional functionality from the crate and strip it down to the bare bones
functionality.

```shell
$ cargo build -p wasmtime-c-api --release --no-default-features
$ ls -lh ./target/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 2.1M Dec 12 07:47 target/release/libwasmtime.so
```

Note that this library is stripped to the bare minimum of functionality which
notably means it does not have a compiler for WebAssembly files. This means that
compilation is no longer supported meaning that `*.cwasm` files must used to
create a module. Additionally error messages will be worse in this mode as less
contextual information is provided.

The final Wasmtime-specific optimization you can apply is to disable logging
statements. Wasmtime and its dependencies make use of the [`log`
crate](https://docs.rs/log) and [`tracing` crate](https://docs.rs/tracing) for
debugging and diagnosing. For a minimal build this isn't needed though so this
can all be disabled through Cargo features to shave off a small amount of code.
Note that for custom embeddings you'd need to replicate the `disable-logging`
feature which sets the `max_level_off` feature for the `log` and `tracing`
crate.

```shell
$ cargo build -p wasmtime-c-api --release --no-default-features --features disable-logging
$ ls -lh ./target/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 2.1M Dec 12 07:49 target/release/libwasmtime.so
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
$ cargo build -p wasmtime-c-api --release --no-default-features --features disable-logging
$ ls -lh ./target/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 2.4M Dec 12 07:49 target/release/libwasmtime.so
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
$ cargo build -p wasmtime-c-api --release --no-default-features --features disable-logging
$ ls -lh ./target/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 2.0M Dec 12 07:49 target/release/libwasmtime.so
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
$ cargo build -p wasmtime-c-api --release --no-default-features --features disable-logging
$ ls -lh ./target/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 1.2M Dec 12 07:50 target/release/libwasmtime.so
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
$ cargo build -p wasmtime-c-api --release --no-default-features --features disable-logging
$ ls -lh ./target/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 1.2M Dec 12 07:50 target/release/libwasmtime.so
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
$ cargo build -p wasmtime-c-api --release --no-default-features --features disable-logging
$ ls -lh ./target/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 1.2M Dec 12 07:50 target/release/libwasmtime.so
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
$ cargo +nightly build -p wasmtime-c-api --release --no-default-features --features disable-logging
$ ls -lh ./target/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 1.2M Dec 12 07:51 target/release/libwasmtime.so
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
$ cargo +nightly build -p wasmtime-c-api --release --no-default-features --features disable-logging \
    -Z build-std=std,panic_abort --target x86_64-unknown-linux-gnu
$ ls -lh target/x86_64-unknown-linux-gnu/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 941K Dec 12 07:52 target/x86_64-unknown-linux-gnu/release/libwasmtime.so
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
$ cargo +nightly build -p wasmtime-c-api --release --no-default-features --features disable-logging \
    -Z build-std=std,panic_abort --target x86_64-unknown-linux-gnu \
    -Z build-std-features=
$ ls -lh target/x86_64-unknown-linux-gnu/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 784K Dec 12 07:53 target/x86_64-unknown-linux-gnu/release/libwasmtime.so
```

And finally, if you can enable the `panic_immediate_abort` feature of the Rust
standard library to shrink panics even further. Note that this comes at a cost
of making bugs/panics very difficult to debug.

```shell
$ export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
$ export CARGO_PROFILE_RELEASE_PANIC=abort
$ export CARGO_PROFILE_RELEASE_LTO=true
$ export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
$ export CARGO_PROFILE_RELEASE_STRIP=debuginfo
$ export RUSTFLAGS="-Zlocation-detail=none"
$ cargo +nightly build -p wasmtime-c-api --release --no-default-features --features disable-logging \
    -Z build-std=std,panic_abort --target x86_64-unknown-linux-gnu \
    -Z build-std-features=panic_immediate_abort
$ ls -lh target/x86_64-unknown-linux-gnu/release/libwasmtime.so
-rwxrwxr-x 2 alex alex 698K Dec 12 07:54 target/x86_64-unknown-linux-gnu/release/libwasmtime.so
```

## Minimizing further

Above shows an example of taking the default `cargo build` result of 260M down
to a 700K binary for the `libwasmtime.so` binary of the C API. Similar steps
can be done to reduce the size of the `wasmtime` CLI executable as well. This is
currently the smallest size with the source code as-is, but there are more size
reductions which haven't been implemented yet.

This is a listing of some example sources of binary size. Some sources of binary
size may not apply to custom embeddings since, for example, your custom
embedding might already not use WASI and might already not be included.

* Unused functionality in the C API - building `libwasmtime.{a,so}` can show a
  misleading file size because the linker is unable to remove unused code. For
  example `libwasmtime.so` contains all code for the C API but your embedding
  may not be using all of the symbols present so in practice the final linked
  binary will often be much smaller than `libwasmtime.so`. Similarly
  `libwasmtime.a` is forced to contain the entire C API so its size is likely
  much larger than a linked application. For a minimal embedding it's
  recommended to link against `libwasmtime.a` with `--gc-sections` as a linker
  flag and evaluate the size of your own application.

* Formatting strings in Wasmtime - Wasmtime makes extensive use of formatting
  strings for error messages and other purposes throughout the implementation.
  Most of this is intended for debugging and understanding more when something
  goes wrong, but much of this is not necessary for a truly minimal embedding.
  In theory much of this could be conditionally compiled out of the Wasmtime
  project to produce a smaller executable. Just how much of the final binary
  size is accounted for by formatting string is unknown, but it's well known in
  Rust that `std::fmt` is not the slimmest of modules.

* CLI: WASI implementation - currently the CLI includes all of WASI. This
  includes two separate implementations of WASI - one for preview2 and one for
  preview1. This accounts for 1M+ of space which is a significant chunk of the
  remaining ~2M.  While removing just preview2 or preview1 would be easy enough
  with a Cargo feature, the resulting executable wouldn't be able to do
  anything. Something like a [plugin feature for the
  CLI](https://github.com/bytecodealliance/wasmtime/issues/7348), however, would
  enable removing WASI while still being a usable executable. Note that the C
  API's implementation of WASI can be disabled because custom host functionality
  can be provided.

* CLI: Argument parsing - as a command line executable `wasmtime` contains
  parsing of command line arguments which currently uses the `clap` crate. This
  contributes ~200k of binary size to the final executable which would likely
  not be present in a custom embedding of Wasmtime. While this can't be removed
  from Wasmtime it's something to consider when evaluating the size of CI
  artifacts.

* Cranelift vs Winch - the "min" builds on CI exclude Cranelift from their
  binary footprint but this comes at a cost of the final binary not
  supporting compilation of wasm modules. If this is required then no effort
  has yet been put into minimizing the code size of Cranelift itself. One
  possible tradeoff that can be made though is to choose between the Winch
  baseline compiler vs Cranelift. Winch should be much smaller from a compiled
  footprint point of view while not sacrificing everything in terms of
  performance. Note though that Winch is still under development.

Above are some future avenues to take in terms of reducing the binary size of
Wasmtime and various tradeoffs that can be made. The Wasmtime project is eager
to hear embedder use cases/profiles if Wasmtime is not suitable for binary size
reasons today. Please feel free to [open an
issue](https://github.com/bytecodealliance/wasmtime/issues/new) and let us know
and we'd be happy to discuss more how best to handle a particular use case.

# Building Wasmtime for a Custom Platform

Wasmtime supports a wide range of functionality by default on major operating
systems such as Windows, macOS, and Linux, but this functionality is not
necessarily present on all platforms (much less custom platforms). Most of
Wasmtime's features are gated behind either platform-specific configuration
flags or Cargo feature flags. The `wasmtime` crate for example documents
[important crate
features](https://docs.rs/wasmtime/latest/wasmtime/#crate-features) which likely
want to be disabled for custom platforms.

Not all of Wasmtime's features are supported on all platforms, but many are
enabled by default. For example the `parallel-compilation` crate feature
requires the host platform to have threads, or in other words the Rust `rayon`
crate must compile for your platform. If the `parallel-compilation` feature is
disabled, though, then `rayon` won't be compiled. For a custom platform, one of
the first things you'll want to do is to disable the default features of the
`wasmtime` crate (or C API).

Some important features to be aware of for custom platforms are:

* `runtime` - you likely want to enable this feature since this includes the
  runtime to actually execute WebAssembly binaries.

* `cranelift` and `winch` - you likely want to disable these features. This
  primarily cuts down on binary size. Note that you'll need to use `*.cwasm`
  artifacts so wasm files will need to be compiled outside of the target
  platform and transferred to them.

* `signals-based-traps` - without this feature Wasmtime won't rely on host OS
  signals (e.g. segfaults) at runtime and will instead perform manual checks to
  avoid signals. This increases portability at the cost of runtime performance.
  For maximal portability leave this disabled.

When compiling Wasmtime for an unknown platform, for example "not Windows" or
"not Unix", then Wasmtime will need some symbols to be provided by the embedder
to operate correctly. The header file at
[`examples/min-platform/embedding/wasmtime-platform.h`][header] describes the
symbols that the Wasmtime runtime requires to work which your platform will need
to provide. Some important notes about this are:

* `wasmtime_{setjmp,longjmp}` are required for trap handling at this time. These
  are thin wrappers around the standard `setjmp` and `longjmp` symbols you'll
  need to provide. An example implementation [looks like this][jumps]. In the
  future this dependency is likely going to go away as trap handling and
  unwinding is migrated to compiled code (e.g. Cranelift) itself.

* `wasmtime_tls_{get,set}` are required for the runtime to operate. Effectively
  a single pointer of TLS storage is necessary. Whether or not this is actually
  stored in TLS is up to the embedder, for example [storage in `static`
  memory][tls] is ok if the embedder knows it won't be using threads.

* `WASMTIME_SIGNALS_BASED_TRAPS` - if this `#define` is given (e.g. the
  `signals-based-traps` feature was enabled at compile time), then your platform
  must have the concept of virtual memory and support `mmap`-like APIs and
  signal handling. Many APIs in [this header][header] are disabled if
  `WASMTIME_SIGNALS_BASED_TRAPS` is turned off which is why it's more portable,
  but if you enable this feature all of these APIs must be implemented.

You can find an example [in the `wasmtime` repository][example] of building a
minimal embedding. Note that for Rust code you'll be using `#![no_std]` and
you'll need to provide a memory allocator and a panic handler as well. The
memory alloator will likely get hooked up to your platform's memory allocator
and the panic handler mostly just needs to abort.

Building Wasmtime for a custom platform is not a turnkey process right now,
there are a number of points that need to be considered:

* For a truly custom platform you'll probably want to create a [custom Rust
  target](https://docs.rust-embedded.org/embedonomicon/custom-target.html). This
  means that Nightly Rust will be required.

* Wasmtime depends on the availability of a memory allocator (e.g. `malloc`).
  Wasmtime assumes that failed memory allocation aborts execution (except for
  the case of allocating linear memories and growing them).

* Not all features for Wasmtime can be built for custom targets. For example
  WASI support does not work on custom targets. When building Wasmtime you'll
  probably want `--no-default-features` and will then want to incrementally add
  features back in as needed.

The `examples/min-platform` directory has an example of building this minimal
embedding and some necessary steps. Combined with the above features about
producing a minimal build currently produces a 400K library on Linux.

[header]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/min-platform/embedding/wasmtime-platform.h
[jumps]: https://github.com/bytecodealliance/wasmtime/blob/e1307216f2aa74fd60c621c8fa326ba80e2a2f75/examples/min-platform/embedding/wasmtime-platform.c#L60-L72
[tls]: https://github.com/bytecodealliance/wasmtime/blob/e1307216f2aa74fd60c621c8fa326ba80e2a2f75/examples/min-platform/embedding/wasmtime-platform.c#L144-L150
[example]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/min-platform/README.md
