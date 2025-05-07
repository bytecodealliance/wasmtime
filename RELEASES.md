## 33.0.0

Unreleased.

### Added

* Cranelift now has initial support for `try_call` and `try_call_indirect`
  instructions, to be used in the future for the WebAssembly exception-handling
  proposal. Wasmtime does not yet implement this proposal yet.
  [#10510](https://github.com/bytecodealliance/wasmtime/pull/10510)
  [#10557](https://github.com/bytecodealliance/wasmtime/pull/10557)
  [#10593](https://github.com/bytecodealliance/wasmtime/pull/10593)

* Cranelift can now optimize some simple possibly-side-effectful instructions,
  such as division.
  [#10524](https://github.com/bytecodealliance/wasmtime/pull/10524)

* Wasmtime now supports `--invoke` for components using the WAVE format.
  [#10054](https://github.com/bytecodealliance/wasmtime/pull/10054)

* Initial support for the Component Model has landed in Wasmtime's C API. Note
  that the API is not yet feature-complete, however.
  [#10566](https://github.com/bytecodealliance/wasmtime/pull/10566)
  [#10598](https://github.com/bytecodealliance/wasmtime/pull/10598)
  [#10651](https://github.com/bytecodealliance/wasmtime/pull/10651)
  [#10675](https://github.com/bytecodealliance/wasmtime/pull/10675)

* Wasmtime's C++ API is now available from this repository and the
  bytecodealliance/wasmtime-cpp repository has been archived. Additionally the
  monolithic `wasmtime.hh` header file has been split into separate header
  files.
  [#10582](https://github.com/bytecodealliance/wasmtime/pull/10582)
  [#10600](https://github.com/bytecodealliance/wasmtime/pull/10600)

* Wasmtime's cookbook-style documentation has been expanded.
  [#10630](https://github.com/bytecodealliance/wasmtime/pull/10630)

* Wasmtime's now supports custom yield behavior when using epoch interrupts.
  [#10671](https://github.com/bytecodealliance/wasmtime/pull/10671)

### Changed

* Wasmtime's bindgen now type-checks export functions in the constructor of
  the generated `{Worldname}Pre` or `{Worldname}` structs, rather than at the
  call of the export function.
  [#10610](https://github.com/bytecodealliance/wasmtime/pull/10610)

* Wasmtime's `component::Component` and `component::Instance` now have consistient
  `get_export` and `get_export_index` methods, which return `(ComponentItem,
  ComponentExportIndex)` and `ComponentExportIndex`, respectively.
  [#10597](https://github.com/bytecodealliance/wasmtime/pull/10597)

* On failure, `wasmtime serve` gives an internal server error response, rather
  than closing the connection.
  [#10645](https://github.com/bytecodealliance/wasmtime/pull/10645)

* Cranelift's single-pass allocator has been disabled due to being unable to
  support internal refactorings in preparation for the WebAssembly exceptions
  proposal. Re-enabling this allocator is tracked at
  [regalloc2#217](https://github.com/bytecodealliance/regalloc2/issues/217) for
  those interested.
  [#10554](https://github.com/bytecodealliance/wasmtime/pull/10554)

* Wasmtime's `{Array,Extern,Struct}Ref` functions will now automatically trigger
  a GC.
  [#10560](https://github.com/bytecodealliance/wasmtime/pull/10560)

* Wasmtime's GC heaps now use the same translation techniques as linear memories
  meaning they have far fewer bounds-checks than before.
  [#10503](https://github.com/bytecodealliance/wasmtime/pull/10503)

* Wasmtime's implementation of WASIp2 has moved to `wasmtime_wasi::p2` from the
  root of the crate.
  [#10073](https://github.com/bytecodealliance/wasmtime/pull/10073)

* Wasmtime will no longer emit calls to Cranelift-defined "libcalls" and instead
  everything goes through Wasmtime's libcall mechanism instead, paving the way
  for a future change for more efficient stack limit checking in wasm.
  [#10657](https://github.com/bytecodealliance/wasmtime/pull/10657)

* Configuration of caching can now be done through an API instead of exclusively
  through a configuration file. Additionally cache-related APIs in `Config` have
  changed.
  [#10665](https://github.com/bytecodealliance/wasmtime/pull/10665)

* Resources in the Component Model are now stored in a single table per-instance
  instead of per-type tables. Guests will see a different pattern of index
  allocation but this is not expected to cause any issues at runtime.
  [#10701](https://github.com/bytecodealliance/wasmtime/pull/10701)

### Fixed

* Some math intrinsics have been fixed when compiled by Rust 1.87+.
  [#10534](https://github.com/bytecodealliance/wasmtime/pull/10534)

* Component model libcalls correctly handle platform-specific argument extension
  in ABIs.
  [#10540](https://github.com/bytecodealliance/wasmtime/pull/10540)

* An off-by-one issue with DWARF debuginfo has been fixed.
  [#10570](https://github.com/bytecodealliance/wasmtime/pull/10570)

* The `Config::target` method is no longer gated by a `#[cfg]` for an enabled
  compiler, it can be used when only the `runtime` feature is available.
  [#10618](https://github.com/bytecodealliance/wasmtime/pull/10618)

* An issue with "simulated" DWARF has been fixed.
  [#10681](https://github.com/bytecodealliance/wasmtime/pull/10681)

* C/C++ headers are now tested that they can be included in isolation, and a
  number of issues have been fixed.
  [#10694](https://github.com/bytecodealliance/wasmtime/pull/10694)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
