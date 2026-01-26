## 36.0.5

Released 2026-01-26.

### Fixed

* Fixed a bug in lowering of `f64.copysign` on x86-64 whereby when combined
  with an `f64.load`, the resulting machine code could read 16 bytes rather
  than 8 bytes. This could result in a segfault when Wasmtime is configured
  without signals-based traps.

--------------------------------------------------------------------------------

## 36.0.4

Released 2026-01-14.

### Fixed

* A possible stack overflow in the x64 backend with `cmp` emission has been
  fixed.
  [#12333](https://github.com/bytecodealliance/wasmtime/pull/12333)

--------------------------------------------------------------------------------

## 36.0.3

Released 2025-11-11.

### Fixed

* Prevent using shared memories with `Memory`.
  [CVE-2025-64345](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-hc7m-r6v8-hg9q)

--------------------------------------------------------------------------------

## 36.0.2

Released 2025-08-26.

### Fixed

* Wasmtime will no longer panic in the pooling allocator when in near-OOM
  conditions related to resetting the linear memory of a slot.
  [#11510](https://github.com/bytecodealliance/wasmtime/pull/11510)

--------------------------------------------------------------------------------

## 36.0.1

Released 2025-08-21.

### Added

* Accessors for internal WASI-related contexts are added to
  `wasmtime_wasi::WasiCtx` to account for refactorings that happened in this
  release.
  [#11473](https://github.com/bytecodealliance/wasmtime/pull/11473)

### Changed

* Release artifacts for the C API are now smaller than the previous release to
  assist with redistribution as-is.
  [#11483](https://github.com/bytecodealliance/wasmtime/pull/11483)

--------------------------------------------------------------------------------

## 36.0.0

Released 2025-08-20.

### Added

* Cranelift's has initial support for inlining between functions. Wasmtime
  additionally now has support for inlining as well, for example between modules
  in a component.
  [#11210](https://github.com/bytecodealliance/wasmtime/pull/11210)
  [#11239](https://github.com/bytecodealliance/wasmtime/pull/11239)
  [#11228](https://github.com/bytecodealliance/wasmtime/pull/11228)
  [#11269](https://github.com/bytecodealliance/wasmtime/pull/11269)
  [#11283](https://github.com/bytecodealliance/wasmtime/pull/11283)

* The async proposal for the Component Model is now fully implemented in
  Wasmtime with a number of WASIp3 interfaces implemented. The implementation
  is still off-by-default and the implementation of WASIp3 is not fully
  complete, but is remains suitable for testing.
  [#11127](https://github.com/bytecodealliance/wasmtime/pull/11127)
  [#11136](https://github.com/bytecodealliance/wasmtime/pull/11136)
  [#11137](https://github.com/bytecodealliance/wasmtime/pull/11137)
  [#11238](https://github.com/bytecodealliance/wasmtime/pull/11238)
  [#11221](https://github.com/bytecodealliance/wasmtime/pull/11221)
  [#11250](https://github.com/bytecodealliance/wasmtime/pull/11250)
  [#11257](https://github.com/bytecodealliance/wasmtime/pull/11257)
  [#11291](https://github.com/bytecodealliance/wasmtime/pull/11291)
  [#11325](https://github.com/bytecodealliance/wasmtime/pull/11325)

### Changed

* Users who implemented `WasiHttpView::is_forbidden_header` from
  `wasmtime-wasi-http` now need to include `DEFAULT_FORBIDDEN_HEADERS`, e.g.
  `DEFAULT_FORBIDDEN_HEADERS.contains(name) || name.as_str() ==
  "custom-forbidden-header"`
  [#11292](https://github.com/bytecodealliance/wasmtime/pull/11292)

* Cranelift's incremental cache has received some optimizations.
  [#11186](https://github.com/bytecodealliance/wasmtime/pull/11186)

* Wasmtime's internal implementations of WebAssembly primitives has been
  refactored to be modeled with safer internal primitives.
  [#11211](https://github.com/bytecodealliance/wasmtime/pull/11211)
  [#11212](https://github.com/bytecodealliance/wasmtime/pull/11212)
  [#11216](https://github.com/bytecodealliance/wasmtime/pull/11216)
  [#11229](https://github.com/bytecodealliance/wasmtime/pull/11229)
  [#11215](https://github.com/bytecodealliance/wasmtime/pull/11215)
  [#11254](https://github.com/bytecodealliance/wasmtime/pull/11254)
  [#11255](https://github.com/bytecodealliance/wasmtime/pull/11255)
  [#11319](https://github.com/bytecodealliance/wasmtime/pull/11319)
  [#11320](https://github.com/bytecodealliance/wasmtime/pull/11320)

* Detection of native hardware features has been refactored on s390x.
  [#11220](https://github.com/bytecodealliance/wasmtime/pull/11220)

* Further progress has been made towards an implementation of the WebAssembly
  exceptions proposal, although it is not yet complete.
  [#11230](https://github.com/bytecodealliance/wasmtime/pull/11230)
  [#11321](https://github.com/bytecodealliance/wasmtime/pull/11321)

* Cranelift's assembler for x64 now supports EVEX encoding.
  [#11153](https://github.com/bytecodealliance/wasmtime/pull/11153)
  [#11270](https://github.com/bytecodealliance/wasmtime/pull/11270)
  [#11303](https://github.com/bytecodealliance/wasmtime/pull/11303)

* The default implementation of `send_request` in the `wasmtime-wasi-http` crate
  is now behind an on-by-default feature gate.
  [#11323](https://github.com/bytecodealliance/wasmtime/pull/11323)

* Configuration of the `bindgen!` macro has been redesigned to more consistently
  configure per-function options such as whether or not it's async.
  [#11328](https://github.com/bytecodealliance/wasmtime/pull/11328)

* Initial support fo `mutatis` has been added to Wasmtime's fuzzers.
  [#11290](https://github.com/bytecodealliance/wasmtime/pull/11290)

* The `debug-builtins` crate feature of `wasmtime` no compiles on `no_std`
  targets.
  [#11304](https://github.com/bytecodealliance/wasmtime/pull/11304)

### Fixed

* Deserializing external modules no long unnecessarily requires the allocation
  to be aligned.
  [#11306](https://github.com/bytecodealliance/wasmtime/pull/11306)

* A CMake linker error and warning when using the C API on macOS has been fixed.
  [#11293](https://github.com/bytecodealliance/wasmtime/pull/11293)
  [#11315](https://github.com/bytecodealliance/wasmtime/pull/11315)

* The C API declaration of `wasmtime_component_linker_instance_add_func` has
  been fixed.
  [#11327](https://github.com/bytecodealliance/wasmtime/pull/11327)

* The calculation of reachable DWARF has been fixed.
  [#11338](https://github.com/bytecodealliance/wasmtime/pull/11338)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
