# Platform Support

The `wasmtime` project is a configurable and lightweight runtime for WebAssembly
which has a number of ways it can be configured. Not all features are supported
on all platforms, but it is intended that `wasmtime` can run in some capacity on
almost all platforms! The matrix of what's being tested, what works, and what's
supported where is evolving over time, and this document hopes to capture a
snapshot of what the current state of the world looks like.

All features of `wasmtime` should work on the following platforms:

* Linux x86\_64
* Linux aarch64
* macOS x86\_64
* Windows x86\_64

For more detailed information about supported platforms, please check out the
sections below!

## JIT compiler support

The JIT compiler, backed by Cranelift, supports the x86\_64 and aarch64
architectures at this time. Support for at least ARM and x86 is planned as well.

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
we'd love to hear about your use case! Feel free to [open an
issue](https://github.com/bytecodealliance/wasmtime/issues/new) on the
`wasmtime` repository to discuss this.

This is a common question we are asked, however, so to provide some more context
on why Wasmtime is the way it is, here's some responses to frequent points
raised about `#![no_std]`:

* **What if my platform doesn't have `std`?** - For platforms without support
  for the Rust standard library the JIT compiler of Wasmtime often won't run on
  the platform as well. The JIT compiler requires `mmap` (or an equivalent), and
  presence of `mmap` often implies presence of a libc which means Rust's `std`
  library works.

  Cargo's [`-Z build-std` feature][zbuild-std] feature is also intended to help
  easily build the standard library for all platforms. With this feature you can
  recompile the standard library (using Nightly Rust for now) with a [custom
  target specification][custom-target] if necessary. Additionally the intention
  at this time is to get `std` building for all platforms, regardless of what
  the platform actually supports. This change is taking time to implement, but
  [rust-lang/rust#74033] is an example of this support growing over time.

  We're also interested in running Wasmtime without a JIT compiler in the
  future, but that is not implemented at this time. Implementing this will
  require a lot more work than tagging crates `#![no_std]`. The Wasmtime
  developers are also very interested in supporting as many targets as possible,
  so if Wasmtime doesn't work on your platform yet we'd love to learn why and
  what we can do to support that platform, but the conversation here is
  typically more nuanced than simply making `wasmtime` compile without `std`.

* **Doesn't `#![no_std]` have smaller binary sizes?** - There's a lot of factors
  that affect binary size in Rust. Compilation options are a huge one but beyond
  that idioms and libraries linked matter quite a lot as well. Code is not
  inherently large when using `std` instead of `core`, it's just that often code
  using `std` has more dependencies (like `std::thread`) which requires code to
  bind. Code size improvements can be made to code using `std` and `core`
  equally, and switching to `#![no_std]` is not a silver bullet for compile
  sizes.

* **The patch to switch to `#![no_std]` is small, why not accept it?** - PRs to
  switch to `#![no_std]` are often relatively small or don't impact too many
  parts of the system. There's a lot more to developing a `#![no_std]`
  WebAssembly runtime than switching a few crates, however. Maintaining a
  `#![no_std]` library over time has a number of costs associated with it:

  * Rust has no stable way to diagnose `no_std` errors in an otherwise `std`
    build, which means that to supoprt this feature it must be tested on CI with
    a `no_std` target. This is costly in terms of CI time, CI maintenance, and
    developers having to do extra builds to avoid CI errors. Note that this
    isn't *more* costly than any other platform supported by Wasmtime, but it's
    a cost nonetheless.

  * Idioms in `#![no_std]` are quite different than normal Rust code. You'll
    import from different crates (`core` instead of `std`) and data structures
    have to all be manually imported from `alloc`. These idioms are difficult to
    learn for newcomers to the project and are not well documented in the
    ecosystem. This cost of development and maintenance is not unique to
    Wasmtime but in general affects the `#![no_std]` ecosystem at large,
    unfortunately.

  * Currently Wasmtime does not have a target use case which requires
    `#![no_std]` support, so it's hard to justify these costs of development.
    We're very interested in supporting as many use cases and targets as
    possible, but the decision to support a target needs to take into account
    the costs associated so we can plan accordingly. Effectively we need to have
    a goal in mind instead of taking on the costs of `#![no_std]` blindly.

  * At this time it's not clear whether `#![no_std]` will be needed long-term,
    so eating short-term costs may not pay off in the long run. Features like
    Cargo's [`-Z build-std`][zbuild-std] may mean that `#![no_std]` is less and
    less necessary over time.

* **How can Wasmtime support `#![no_std]` if it uses X?** - Wasmtime as-is today
  is not suitable for many `#![no_std]` contexts. For example it might use
  `mmap` for allocating JIT code memory, leverage threads for caching, or use
  thread locals when calling into JIT code. These features are difficult to
  support in their full fidelity on all platforms, but the Wasmtime developers
  are very much aware of this! Wasmtime is intended to be configurable where
  many of these features are compile-time or runtime options. For example caches
  can be disabled, JITs can be removed and replaced with interpreters, or users
  could provide a callback to allocate memory instead of using the OS.
  This is sort of a long-winded way of saying that Wasmtime on the surface may
  today look like it won't support `#![no_std]`, but this is almost always
  simply a matter of time and development priorities rather than a fundamental
  reason why Wasmtime *couldn't* support `#![no_std]`.

Note that at this time these guidelines apply not only to Wasmtime but also to
some of its dependencies developed by the Bytecode Alliance such as the
[wasm-tools repository](https://github.com/bytecodealliance/wasm-tools). These
projects don't have the same runtime requirements as Wasmtime (e.g. `wasmparser`
doesn't need `mmap`), but we're following the same guidelines above at this
time. Patches to add `#![no_std]`, while possibly small, incur many of the same
costs and also have an unclear longevity as features like [`-Z
build-std`][zbuild-std] evolve.

[zbuild-std]: https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#build-std
[custom-target]: https://doc.rust-lang.org/rustc/targets/custom.html
[rust-lang/rust#74033]: https://github.com/rust-lang/rust/pull/74033
