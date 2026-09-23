## 49.0.1

Released 2026-09-24.

### Fixed

* Do not drop fuel-spend accrued by callees of `call_ref` and callees that
  return via exception throws.
  [GHSA-m63x-6p34-q65x](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-m63x-6p34-q65x)

* Fix fuel spend for dynamic record lifting.
  [GHSA-jqpg-j7w6-42pr](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-jqpg-j7w6-42pr)

## 49.0.0

Released 2026-09-21.

### Added

* Winch now has initial experimental support for the WebAssembly
  exception-handling proposal.
  [#14066](https://github.com/bytecodealliance/wasmtime/pull/14066)
  [#14081](https://github.com/bytecodealliance/wasmtime/pull/14081)
  [#14132](https://github.com/bytecodealliance/wasmtime/pull/14132)
  [#14149](https://github.com/bytecodealliance/wasmtime/pull/14149)
  [#14180](https://github.com/bytecodealliance/wasmtime/pull/14180)
  [#14209](https://github.com/bytecodealliance/wasmtime/pull/14209)

* Wasmtime's type reflection APIs for WebAssembly GC types now have dedicated
  enums for `Heap{Top,Bottom}Type`s.
  [#14208](https://github.com/bytecodealliance/wasmtime/pull/14208)

* Wasmtime now supports the use of environment variables to configure most CLI
  arguments in addition to the preexisting CLI flags.
  [#14217](https://github.com/bytecodealliance/wasmtime/pull/14217)

* Cranelift will now fold `u{add,mul}_overflow` results combined with `brif`
  instructions for more optimal codegen.
  [#14217](https://github.com/bytecodealliance/wasmtime/pull/14217)
  [#14254](https://github.com/bytecodealliance/wasmtime/pull/14254)

* Components can be tested for equality with a new `Component::same` API.
  [#14257](https://github.com/bytecodealliance/wasmtime/pull/14257)

* The Wasmtime C/C++ API now supports the `component-model-implements` feature.
  [#14264](https://github.com/bytecodealliance/wasmtime/pull/14264)

* Cranelift will now emit the `{u,s}bfm` instructions on AArch64.
  [#14187](https://github.com/bytecodealliance/wasmtime/pull/14187)

### Changed

* The wide-arithmetic WebAssembly proposal is now enabled by default.
  [#14110](https://github.com/bytecodealliance/wasmtime/pull/14110)

* `Config::operator_cost` now applies to operators inside constant expressions
  (global initializers, element and data segment offsets, element segment
  expressions) and to the synthesized call to a module's `start` function.
  Previously each of those was charged 1 fuel unit regardless of the configured
  cost.
  [#14215](https://github.com/bytecodealliance/wasmtime/pull/14215)

* Wasmtime's WASI implementation will no longer over-read stdin where possible.
  [#14077](https://github.com/bytecodealliance/wasmtime/pull/14077)

* Wasmtime's WASIp2 implementation now maps host-level "broken pipe" errors to
  `StreamError::Closed`.
  [#14107](https://github.com/bytecodealliance/wasmtime/pull/14107)

* Wasmtime's WASIp2 implementation of `wasi:http` now validates ports in the
  same manner as wasip3.
  [#14123](https://github.com/bytecodealliance/wasmtime/pull/14123)

* Wasmtime's WASI implementation of `wasi:filesystem` now returns the
  `is-directory` error in some situations when a file is provided.
  [#14135](https://github.com/bytecodealliance/wasmtime/pull/14135)

* The `wasmtime-wasi-http` crate now has the `p3` feature enabled by default.
  [#14137](https://github.com/bytecodealliance/wasmtime/pull/14137)

* Wasmtime's WASIp2 implementation of `wasi:http` now runs more hooks from the
  `WasiHttpHooks` trait in the same manner as wasip3.
  [#14167](https://github.com/bytecodealliance/wasmtime/pull/14167)

* Wasmtime now requires Rust 1.96.0 to compile.
  [#14184](https://github.com/bytecodealliance/wasmtime/pull/14184)

* Adjustments have been made to the scheduling of threads w.r.t. trapping
  behavior for cooperatively threaded components. These changes are made to
  align with the upstream component-model specification.
  [#14146](https://github.com/bytecodealliance/wasmtime/pull/14146)
  [#14250](https://github.com/bytecodealliance/wasmtime/pull/14250)

* Fuel for bulk operations is now consumed after the bulk operation has
  completed instead of up-front, restoring previous behavior where a failed very
  large memory growth doesn't trap the original program.
  [#14213](https://github.com/bytecodealliance/wasmtime/pull/14213)

* A number of future `poll` calls throughout the WASIp2 implementation now
  explicitly opt-out of Tokio's per-task cooperative budget to better uphold
  guarantees of WASIp2 APIs.
  [#14265](https://github.com/bytecodealliance/wasmtime/pull/14265)

* The `run_concurrent` function is now allowed to run recursively for separate
  stores, but it still can only be used at most once recursively for any one
  store.
  [#14302](https://github.com/bytecodealliance/wasmtime/pull/14302)

* The compile-time performance of the alias analysis in Cranelift has been
  greatly improved.
  [#14306](https://github.com/bytecodealliance/wasmtime/pull/14306)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
* [48.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-48.0.0/RELEASES.md)
* [47.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-47.0.0/RELEASES.md)
* [46.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-46.0.0/RELEASES.md)
* [45.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-45.0.0/RELEASES.md)
* [44.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-44.0.0/RELEASES.md)
* [43.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-43.0.0/RELEASES.md)
* [42.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-42.0.0/RELEASES.md)
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
