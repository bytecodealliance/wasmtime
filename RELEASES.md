## 24.0.0

Released 2024-08-20.

### Added

* A new `wasmtime_engine_clone` function was added to the C API.
  [#8907](https://github.com/bytecodealliance/wasmtime/pull/8907)

* Wasmtime now has basic support for allocating a `StructRef` in the embedder
  API.
  [#8933](https://github.com/bytecodealliance/wasmtime/pull/8933)

* The `wasmtime run` subcommand now support a `--argv0` flag indicating the
  value of the first element to arguments reported to wasm if it shouldn't be
  the default of the wasm binary name itself.
  [#8961](https://github.com/bytecodealliance/wasmtime/pull/8961)

* Support for Winch on AArch64 continued to improve.
  [#8921](https://github.com/bytecodealliance/wasmtime/pull/8921)
  [#9018](https://github.com/bytecodealliance/wasmtime/pull/9018)
  [#9033](https://github.com/bytecodealliance/wasmtime/pull/9033)
  [#9051](https://github.com/bytecodealliance/wasmtime/pull/9051)

* An initial implementation of the `wasi-runtime-config` proposal was added to
  Wasmtime.
  [#8950](https://github.com/bytecodealliance/wasmtime/pull/8950)
  [#8970](https://github.com/bytecodealliance/wasmtime/pull/8970)
  [#8981](https://github.com/bytecodealliance/wasmtime/pull/8981)

* Initial support for f16 and f128 in Cranelift continued to improve.
  [#8893](https://github.com/bytecodealliance/wasmtime/pull/8893)
  [#9045](https://github.com/bytecodealliance/wasmtime/pull/9045)

* More types in `wasmtime-wasi-http` implement the `Debug` trait.
  [#8979](https://github.com/bytecodealliance/wasmtime/pull/8979)

* The `wasmtime explore` subcommand now supports exploring CLIF too.
  [#8972](https://github.com/bytecodealliance/wasmtime/pull/8972)

* Support for SIMD in Winch has begun, but it is not complete yet.
  [#8990](https://github.com/bytecodealliance/wasmtime/pull/8990)
  [#9006](https://github.com/bytecodealliance/wasmtime/pull/9006)

* Initial work on Pulley, an interpreter for Wasmtime, has begun.
  [#9008](https://github.com/bytecodealliance/wasmtime/pull/9008)
  [#9013](https://github.com/bytecodealliance/wasmtime/pull/9013)
  [#9014](https://github.com/bytecodealliance/wasmtime/pull/9014)

* The `-Wunknown-imports-trap` flag to `wasmtime run` now supports components.
  [#9021](https://github.com/bytecodealliance/wasmtime/pull/9021)

* An initial implementation of the `wasi-keyvalue` proposal was added to
  Wasmtime.
  [#8983](https://github.com/bytecodealliance/wasmtime/pull/8983)
  [#9032](https://github.com/bytecodealliance/wasmtime/pull/9032)
  [#9050](https://github.com/bytecodealliance/wasmtime/pull/9050)
  [#9062](https://github.com/bytecodealliance/wasmtime/pull/9062)

* An `unsafe` API has been added to unload process trap handlers.
  [#9022](https://github.com/bytecodealliance/wasmtime/pull/9022)

* The s390x backend now fully supports tail calls.
  [#9052](https://github.com/bytecodealliance/wasmtime/pull/9052)

### Changed

* The `flags` type in the component model now has a hard limit of 32-or-fewer
  flags. For more information about this transition see
  https://github.com/WebAssembly/component-model/issues/370.
  [#8882](https://github.com/bytecodealliance/wasmtime/pull/8882)

* Multiple returns for functions in the component model are now gated by default
  and are planned to be removed.
  [#8965](https://github.com/bytecodealliance/wasmtime/pull/8965)

* TCP streams in WASIp2 will now immediately return `StreamError::Closed` when
  the TCP stream is closed or shut down.
  [#8968](https://github.com/bytecodealliance/wasmtime/pull/8968)
  [#9055](https://github.com/bytecodealliance/wasmtime/pull/9055)

* Cranelift will now perform constant propagation on some floating-point
  operations.
  [#8954](https://github.com/bytecodealliance/wasmtime/pull/8954)

* Wasmtime and Cranelift now require at least Rust 1.78.0 to compile.
  [#9010](https://github.com/bytecodealliance/wasmtime/pull/9010)

* The `wasmtime::Val` type now implements the `Copy` trait.
  [#9024](https://github.com/bytecodealliance/wasmtime/pull/9024)

* Wasmtime's wasi-nn implementation has been updated to track the upstream
  specification.
  [#9056](https://github.com/bytecodealliance/wasmtime/pull/9056)

* Names provided to `trappable_imports` in `bindgen!` are now validated to be
  used.
  [#9057](https://github.com/bytecodealliance/wasmtime/pull/9057)

* Support for multi-package `*.wit` files now requires a `package ...;` header
  at the top of the file.
  [#9053](https://github.com/bytecodealliance/wasmtime/pull/9053)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
* [23.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-23.0.0/RELEASES.md)
* [22.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-22.0.0/RELEASES.md)
* [21.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-21.0.0/RELEASES.md)
* [20.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-20.0.0/RELEASES.md)
* [19.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-19.0.0/RELEASES.md)
* [18.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-18.0.0/RELEASES.md)
* [17.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-17.0.0/RELEASES.md)
* [16.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-16.0.0/RELEASES.md)
* [15.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-15.0.0/RELEASES.md)
* [14.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-14.0.0/RELEASES.md)
* [13.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-13.0.0/RELEASES.md)
* [12.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-12.0.0/RELEASES.md)
* [11.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-11.0.0/RELEASES.md)
* [10.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-10.0.0/RELEASES.md)
* [9.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-9.0.0/RELEASES.md)
* [8.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-8.0.0/RELEASES.md)
* [7.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-7.0.0/RELEASES.md)
* [6.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-6.0.0/RELEASES.md)
* [5.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-5.0.0/RELEASES.md)
* [4.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-4.0.0/RELEASES.md)
* [3.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-3.0.0/RELEASES.md)
* [2.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-2.0.0/RELEASES.md)
* [1.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-1.0.0/RELEASES.md)
* [0.40.x](https://github.com/bytecodealliance/wasmtime/blob/release-0.40.0/RELEASES.md)
* [0.39.x](https://github.com/bytecodealliance/wasmtime/blob/release-0.39.0/RELEASES.md)
* [0.38.x](https://github.com/bytecodealliance/wasmtime/blob/release-0.38.0/RELEASES.md)
* [0.37.x](https://github.com/bytecodealliance/wasmtime/blob/release-0.37.0/RELEASES.md)
* [0.36.x](https://github.com/bytecodealliance/wasmtime/blob/release-0.36.0/RELEASES.md)
* [0.35.x](https://github.com/bytecodealliance/wasmtime/blob/release-0.35.0/RELEASES.md)
* [0.34.x](https://github.com/bytecodealliance/wasmtime/blob/release-0.34.0/RELEASES.md)
* [0.33.x](https://github.com/bytecodealliance/wasmtime/blob/release-0.33.0/RELEASES.md)
* [0.32.x (and prior)](https://github.com/bytecodealliance/wasmtime/blob/release-0.32.0/RELEASES.md)
