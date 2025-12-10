## 40.0.0

Unreleased.

### Added

* WASIp3 support for `wasi:http` now implements `Response::from_http` to convert
  from standard Rust types to WASI types.
  [#12063](https://github.com/bytecodealliance/wasmtime/pull/12063)

* Cranelift now supports a "patchable" ABI which has a maximum number of
  arguments and clobbers no registers. This is paired as well with a new
  `patchable_call` instruction which supports being turned into NOPs at runtime.
  [#12061](https://github.com/bytecodealliance/wasmtime/pull/12061)
  [#12101](https://github.com/bytecodealliance/wasmtime/pull/12101)

### Changed

* Support for the WebAssembly `threads` proposal is now classified as tier 2 by
  default. Additionally creation of `SharedMemory` is disabled by deafult behind
  a new config knob/CLI flag.
  [#12036](https://github.com/bytecodealliance/wasmtime/pull/12036)

* A variety of peephole-style optimizations have been added to Cranelift's
  optimization passes.
  [#11994](https://github.com/bytecodealliance/wasmtime/pull/11994)
  [#11995](https://github.com/bytecodealliance/wasmtime/pull/11995)
  [#11996](https://github.com/bytecodealliance/wasmtime/pull/11996)
  [#11997](https://github.com/bytecodealliance/wasmtime/pull/11997)
  [#11998](https://github.com/bytecodealliance/wasmtime/pull/11998)
  [#11999](https://github.com/bytecodealliance/wasmtime/pull/11999)
  [#12000](https://github.com/bytecodealliance/wasmtime/pull/12000)
  [#12006](https://github.com/bytecodealliance/wasmtime/pull/12006)
  [#12008](https://github.com/bytecodealliance/wasmtime/pull/12008)

* Component host functions have been slightly optimized to remove an `Arc` clone
  and reduce contention.
  [#11987](https://github.com/bytecodealliance/wasmtime/pull/11987)

* Support for component-model-async has been updated to account for the
  changes specified in [WebAssembly/component-model#578](https://github.com/WebAssembly/component-model/pull/578).
  This means that historical binaries using WASIp3, for example, are no longer
  valid. Recompilation of historical components will be required and
  source-level changes may also be required in some circumstances.
  [#12031](https://github.com/bytecodealliance/wasmtime/pull/12031)
  [#12043](https://github.com/bytecodealliance/wasmtime/pull/12043)

* The `UnsyncBoxBody` type is now used everywhere in wasmtime-wasi-http instead
  of just in the wasip3 support.
  [#12060](https://github.com/bytecodealliance/wasmtime/pull/12060)

* Initial groundwork for gracefully handling OOM (e.g. returning an error
  instead of aborting) has been added.
  [#12070](https://github.com/bytecodealliance/wasmtime/pull/12070)
  [#12089](https://github.com/bytecodealliance/wasmtime/pull/12089)

* Wasmtime will create a private copy of code memory when guest debugging is
  enabled to assist with modifying code when adding/removing breakpoints.
  [#12051](https://github.com/bytecodealliance/wasmtime/pull/12051)

* The `ResourceTable` type will no longer use `Tombstone` when compiled in debug
  mode.
  [#12114](https://github.com/bytecodealliance/wasmtime/pull/12114)

* Intra-component future/stream reads/writes will now trap instead of
  accidentally being allowed.
  [#12117](https://github.com/bytecodealliance/wasmtime/pull/12117)

* Cranelift optimization rules have been tweaked after it was discovered that
  they could pessimize code containing long chains of computations.
  [#12116](https://github.com/bytecodealliance/wasmtime/pull/12116)

### Fixed

* Compilation of `i8x16.popcnt` has been fixed in Winch for some potential
  inputs.
  [#12010](https://github.com/bytecodealliance/wasmtime/pull/12010)

* A panic in `Instance::prepare_call` for some component-model-async situations
  has been fixed.
  [#12054](https://github.com/bytecodealliance/wasmtime/pull/12054)

* An off-by-one error for lifting/lowering enums/variants with 255 cases has
  been fixed.
  [#12066](https://github.com/bytecodealliance/wasmtime/pull/12066)

* Restarting the read of a host future after cancellation has been fixed.
  [#12093](https://github.com/bytecodealliance/wasmtime/pull/12093)

* Compilation for OpenBSD on x86\_64 has been fixed.
  [#12097](https://github.com/bytecodealliance/wasmtime/pull/12097)

* Components containing a module type which exoprts a `tag` are now supported.
  [#12125](https://github.com/bytecodealliance/wasmtime/pull/12125)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
