# Building a Minimal Wasmtime embedding

Wasmtime embeddings may wish to optimize for binary size and runtime footprint
to fit on a small system. This documentation is intended to guide some features
of Wasmtime and how to best produce a minimal build of Wasmtime.

## Building a minimal CLI

> *Note*: the exact numbers in this section were last updated on 2023-10-18 on a
> macOS aarch64 host. They should provide a general ballpark estimate but should
> be confirmed locally again before being totally relied upon.

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
functionality.

```shell
$ cargo build --release --no-default-features
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
$ cargo build --release --no-default-features --features disable-logging
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
$ cargo build --release --no-default-features --features disable-logging
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
$ cargo build --release --no-default-features --features disable-logging
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
$ cargo build --release --no-default-features --features disable-logging
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
$ cargo build --release --no-default-features --features disable-logging
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
$ cargo build --release --no-default-features --features disable-logging
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
$ cargo +nightly build --release --no-default-features --features disable-logging
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
$ cargo +nightly build --release --no-default-features --features disable-logging \
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
$ cargo +nightly build --release --no-default-features --features disable-logging \
    -Z build-std=std,panic_abort --target aarch64-apple-darwin \
    -Z build-std-features=
$ ls -l ./target/aarch64-apple-darwin/release/wasmtime
-rwxr-xr-x@ 1 root  root    2.1M Oct 18 09:39 target/aarch64-apple-darwin/release/wasmtime
```

## Minimizing further

Above shows an example of taking the default `cargo build` result of 130M down
to a 2.1M binary for the `wasmtime` executable. The remaining space in this
binary is occupied by features which often aren't needed in all embeddings, for
example:

* Command-line argument parsing via the `clap` crate. Custom embeddings likely
  won't use this at all and/or will have their own command line parsing
  elsewhere. In the above 2.1M number this is about ~200k.

* WASI implementations may not all be needed or may be slimmed down. For example
  the above binary contains two implementations of `wasi_snapshot_preview1` at
  this time and removing one of them shaves off around 300k.

Most Wasmtime embeddings are unlikely to be the `wasmtime` CLI itself meaning
that the above sources of size will be eliminated as well in a custom embedding
of the `wasmtime` crate.

If, however, after applying the above optimizations, flags, etc, results in a
binary too large for your use case we'd be quite interested to hear about it!
Please feel free to [open an
issue](https://github.com/bytecodealliance/wasmtime/issues/new) and let us know.
There's still remaining fruit to be picked to minimize Wasmtime's footprint
further and user feedback is helpful to prioritize this work.
