# Wasmtime Releases

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
