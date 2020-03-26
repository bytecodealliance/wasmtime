# Wasmtime Releases

--------------------------------------------------------------------------------

## 0.14.0

Unreleased

--------------------------------------------------------------------------------

## 0.13.0

Released 2020-03-24.

### Added

* Lots of documentation of `wasmtime` has been updated. Be sure to check out the
  [book](https://bytecodealliance.github.io/wasmtime/) and [API
  documentation](https://bytecodealliance.github.io/wasmtime/api/wasmtime/)!

* All wasmtime example programs are now in a top-level `examples` directory and
  are available in both C and Rust.
  [#1286](https://github.com/bytecodealliance/wasmtime/pull/1286)

* A `wasmtime::Linker` type was added to conveniently link link wasm modules
  together and create instances that reference one another.
  [#1384](https://github.com/bytecodealliance/wasmtime/pull/1384)

* Wasmtime now has "jitdump" support enabled by default which allows [profiling
  wasm code on linux][jitdump].
  [#1310](https://github.com/bytecodealliance/wasmtime/pull/1310)

* The `wasmtime::Caller` type now exists as a first-class way to access the
  caller's exports, namely memory, when implementing host APIs. This can be the
  first argument of functions defined with `Func::new` or `Func::wrap` which
  allows easily implementing methods which take a pointer into wasm memory. Note
  that this only works for accessing the caller's `Memory` for now and it must
  be exported. This will eventually be replaced with a more general-purpose
  mechanism like interface types.
  [#1290](https://github.com/bytecodealliance/wasmtime/pull/1290)

* The bulk memory proposal has been fully implemented.
  [#1264](https://github.com/bytecodealliance/wasmtime/pull/1264)
  [#976](https://github.com/bytecodealliance/wasmtime/pull/976)

* Virtual file support has been added to `wasi-common`.
  [#701](https://github.com/bytecodealliance/wasmtime/pull/701)

* The C API has been enhanced with a Wasmtime-specific `wasmtime_wat2wasm` to
  parse `*.wat` files via the C API.
  [#1206](https://github.com/bytecodealliance/wasmtime/pull/1206)

[jitdump]: https://bytecodealliance.github.io/wasmtime/examples-profiling.html

### Changed

* The `wast` and `wasm2obj` standalone binaries have been removed. They're
  available via the `wasmtime wast` and `wasmtime wasm2obj` subcommands.
  [#1372](https://github.com/bytecodealliance/wasmtime/pull/1372)

* The `wasi-common` crate now uses the new `wiggle` crate to auto-generate a
  trait which is implemented for the current wasi snapshot.
  [#1202](https://github.com/bytecodealliance/wasmtime/pull/1202)

* Wasmtime no longer has a dependency on a C++ compiler.
  [#1365](https://github.com/bytecodealliance/wasmtime/pull/1365)

* The `Func::wrapN` APIs have been consolidated into one `Func::wrap` API.
  [#1363](https://github.com/bytecodealliance/wasmtime/pull/1363)

* The `Callable` trait has been removed and now `Func::new` takes a closure
  directly.
  [#1363](https://github.com/bytecodealliance/wasmtime/pull/1363)

* The Cranelift repository has been merged into the Wasmtime repository.

* Support for interface types has been temporarily removed.
  [#1292](https://github.com/bytecodealliance/wasmtime/pull/1292)

* The exit code of the `wasmtime` CLI has changed if the program traps.
  [#1274](https://github.com/bytecodealliance/wasmtime/pull/1274)

* The `wasmtime` CLI now logs to stderr by default and the `-d` flag has been
  renamed to `--log-to-file`.
  [#1266](https://github.com/bytecodealliance/wasmtime/pull/1266)

* Values cannot cross `Store` objects, meaning you can't instantiate a module
  with values from different stores nor pass values from different stores into
  methods.
  [#1016](https://github.com/bytecodealliance/wasmtime/pull/1016)

--------------------------------------------------------------------------------

## 0.12.0

Released 2020-02-26.

### Added

* Support for the [WebAssembly text annotations proposal][annotations-proposal]
  has been added.
  [#998](https://github.com/bytecodealliance/wasmtime/pull/998)

* An initial C API for instantiating WASI modules has been added.
  [#977](https://github.com/bytecodealliance/wasmtime/pull/977)

* A new suite of `Func::getN` functions have been added to the `wasmtime` API to
  call statically-known function signatures in a highly optimized fashion.
  [#955](https://github.com/bytecodealliance/wasmtime/pull/955)

* Initial support for profiling JIT code through perf jitdump has been added.
  [#360](https://github.com/bytecodealliance/wasmtime/pull/360)

* More CLI flags corresponding to proposed WebAssembly features have been added.
  [#917](https://github.com/bytecodealliance/wasmtime/pull/917)

[annotations-proposal]: https://github.com/webassembly/annotations

### Changed

* The `wasmtime` CLI as well as embedding API will optimize WebAssembly code by
  default now.
  [#973](https://github.com/bytecodealliance/wasmtime/pull/973)
  [#988](https://github.com/bytecodealliance/wasmtime/pull/988)

* The `verifier` pass in Cranelift is now no longer run by default when using
  the embedding API.
  [#882](https://github.com/bytecodealliance/wasmtime/pull/882)

### Fixed

* Code caching now accurately accounts for optimization levels, ensuring that if
  you ask for optimized code you're not accidentally handed unoptimized code
  from the cache.
  [#974](https://github.com/bytecodealliance/wasmtime/pull/974)

* Automated releases for tags should be up and running again, along with
  automatic publication of the `wasmtime` Python package.
  [#971](https://github.com/bytecodealliance/wasmtime/pull/971)
