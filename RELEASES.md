## 42.0.0

Released 2026-02-20.

### Added

* Cranelift now supports bitwise operations on floats on aarch64.
  [#12326](https://github.com/bytecodealliance/wasmtime/pull/12326)

* Cranelift now supports NaN canonicalization of f16 and f128.
  [#12337](https://github.com/bytecodealliance/wasmtime/pull/12337)

* Wasmtime has gained minimal support to implement fixed-length lists in the
  component model being communicated between composed components.
  [#10619](https://github.com/bytecodealliance/wasmtime/pull/10619)

* Wasmtime's `Error` and `Result` types are now built-in to the `wasmtime` crate
  and are no longer defined by the `anyhow` crate. Wasmtime exports a
  mostly-compatible `anyhow`-like API at `wasmtime::error` which is used
  instead. Wasmtime's own `Error` handles OOM internally and is foundational
  part of Wasmtime's work-in-progress support to gracefully handle OOM.
  [#12309](https://github.com/bytecodealliance/wasmtime/pull/12309)

* Wasmtime now exports an extension trait to convert `anyhow::Result<T>` into
  `wasmtime::Result<T>`.
  [#12255](https://github.com/bytecodealliance/wasmtime/pull/12255)

* Wasmtime supports a new `bindgen!` option to generate bindings specifically
  with `anyhow::Result` instead of `wasmtime::Result`.
  [#12331](https://github.com/bytecodealliance/wasmtime/pull/12331)

* The Nvidia-Cuda execution provider is now supported for the wasi-nn onnx
  backend.
  [#12044](https://github.com/bytecodealliance/wasmtime/pull/12044)

* Non-exported and private entities can now be accessed through the debugger
  API.
  [#12367](https://github.com/bytecodealliance/wasmtime/pull/12367)

* A new `Store::try_new` API has been added which handles OOM.
  [#12415](https://github.com/bytecodealliance/wasmtime/pull/12415)

* Initial configuration knobs for record-and-replay support have been added.
  [#12375](https://github.com/bytecodealliance/wasmtime/pull/12375)

* Cranelift's s390x backend now has support for instructions added in z17.
  [#12319](https://github.com/bytecodealliance/wasmtime/pull/12319)

* Wasmtime's implementation of fibers can now be compiled on riscv32imac
  platforms.
  [#12506](https://github.com/bytecodealliance/wasmtime/pull/12506)

### Changed

* Reentrance rules for WebAssembly components have changed in accordance with
  upstream specification changes. Embeddings are not expected to be affected,
  but please reach out if you find problems.
  [#12349](https://github.com/bytecodealliance/wasmtime/pull/12349)

* Wasmtime's `Config::async_support` option is now removed and no longer
  necessary. Embeddings can likely just remove turning this on and everything
  should keep working like normal.
  [#12371](https://github.com/bytecodealliance/wasmtime/pull/12371)

* Wasmtime now supports `Config::concurrency_support` as a knob to enable or
  disable `*_concurrent` APIs at runtime when the `component-model-async` crate
  feature is enabled.
  [#12416](https://github.com/bytecodealliance/wasmtime/pull/12416)

* The `post_return`-style functions in Wasmtime's API are now noops and will be
  removed in the future.
  [#12498](https://github.com/bytecodealliance/wasmtime/pull/12498)

* Translation of `global.get` of a defined, immutable global is now turned into
  a CLIF constant.
  [#12234](https://github.com/bytecodealliance/wasmtime/pull/12234)

* The `wasmtime-wasi-nn` crate's dependency on `ort` has been updated.
  [#12162](https://github.com/bytecodealliance/wasmtime/pull/12162)

* Error bounds requiring using `hyper::Error` in `wasmtime-wasi-http` have been
  relaxed to taking `E: Into<ErrorCode>` instead.
  [#12227](https://github.com/bytecodealliance/wasmtime/pull/12227)

* The `cranelift-assembler-x64`, and `cranelift-isle` crates now supports
  no\_std targets. The `cranelift-codegen` crate now mostly supports no\_std,
  but not entirely.
  [#12222](https://github.com/bytecodealliance/wasmtime/pull/12222)
  [#12235](https://github.com/bytecodealliance/wasmtime/pull/12235)
  [#12236](https://github.com/bytecodealliance/wasmtime/pull/12236)

* Android release binaries are now compiled with a larger page size configured.
  [#12246](https://github.com/bytecodealliance/wasmtime/pull/12246)

* Implicit binds are now allowed for WASIp3 sockets.
  [#12225](https://github.com/bytecodealliance/wasmtime/pull/12225)

* Wasmtime's implementation of component-model-async now correctly checks for
  whether tasks are allowed to block in all guest-to-guest situations.
  [#12282](https://github.com/bytecodealliance/wasmtime/pull/12282)

* The `wasmtime-wasi-tls` crate now has an OpenSSL backend.
  [#12228](https://github.com/bytecodealliance/wasmtime/pull/12228)

* Wasmtime's `OutOfMemory` error now keeps track of the attempted allocation
  size that failed.
  [#12351](https://github.com/bytecodealliance/wasmtime/pull/12351)

* Wasmtime's `*_unchecked` functions now work with `MaybeUninit<ValRaw>` instead
  of `ValRaw` directly.
  [#12366](https://github.com/bytecodealliance/wasmtime/pull/12366)

* Wasmtime now requires Rust 1.91.0 to compile.
  [#12392](https://github.com/bytecodealliance/wasmtime/pull/12392)

* Wasmtime's management of component-model-async tasks is now consistently done
  in all boundaries between guest/host tasks.
  [#12379](https://github.com/bytecodealliance/wasmtime/pull/12379)

### Fixed

* The `HeapType::matches` implementation for `NoExn` has been fixed.
  [#12350](https://github.com/bytecodealliance/wasmtime/pull/12350)

* Enum discriminant validation for composed components has been fixed.
  [#12356](https://github.com/bytecodealliance/wasmtime/pull/12356)

* Shifts in some situations on aarch64 with Winch have been fixed.
  [#12501](https://github.com/bytecodealliance/wasmtime/pull/12501)

* Debug value locations around cold blocks have been improved.
  [#12484](https://github.com/bytecodealliance/wasmtime/pull/12484)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
* [41.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-41.0.0/RELEASES.md)
* [40.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-40.0.0/RELEASES.md)
* [39.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-39.0.0/RELEASES.md)
* [38.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-38.0.0/RELEASES.md)
* [37.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-37.0.0/RELEASES.md)
* [36.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-36.0.0/RELEASES.md)
* [35.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-35.0.0/RELEASES.md)
* [34.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-34.0.0/RELEASES.md)
* [33.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-33.0.0/RELEASES.md)
* [32.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-32.0.0/RELEASES.md)
* [31.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-31.0.0/RELEASES.md)
* [30.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-30.0.0/RELEASES.md)
* [29.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-29.0.0/RELEASES.md)
* [28.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-28.0.0/RELEASES.md)
* [27.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-27.0.0/RELEASES.md)
* [26.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-26.0.0/RELEASES.md)
* [25.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-25.0.0/RELEASES.md)
* [24.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-24.0.0/RELEASES.md)
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
