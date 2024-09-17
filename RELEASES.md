## 25.0.0

Unreleased.

### Added

* The WinML backend of wasmtime-wasi-nn now supports FP16 and I64.
  [#8964](https://github.com/bytecodealliance/wasmtime/pull/8964)

* Pooling allocator configuration options for table elements and core instance
  size can now be changed on the CLI.
  [#9138](https://github.com/bytecodealliance/wasmtime/pull/9138)

* Wasmtime now supports the extended-const WebAssembly proposal.
  [#9141](https://github.com/bytecodealliance/wasmtime/pull/9141)

* The `wasmtime` crate embedding API now has `ArrayRef` for allocating wasm GC
  arrays.
  [#9145](https://github.com/bytecodealliance/wasmtime/pull/9145)

* Cranelift now has a `stack_switch` CLIF instruction to be used with the
  WebAssembly stack switching proposal.
  [#9078](https://github.com/bytecodealliance/wasmtime/pull/9078)

* There are now more constructors available on `bindgen!`-generated structures
  for component exports now which use instantiated components rather than
  pre-instantiated components.
  [#9177](https://github.com/bytecodealliance/wasmtime/pull/9177)

### Changed

* Wasmtime's support for WASI is now listed with the 0.2.1 version instead of
  0.2.0. This is expected to not cause fallout or breakage, but please open an
  issue if you see any problems.
  [#9063](https://github.com/bytecodealliance/wasmtime/pull/9063)

* Work continues on Winch's AArch64 backend.
  [#9114](https://github.com/bytecodealliance/wasmtime/pull/9114)
  [#9092](https://github.com/bytecodealliance/wasmtime/pull/9092)
  [#9171](https://github.com/bytecodealliance/wasmtime/pull/9171)

* Component model resource methods can now be generated as `async` and will do
  so by default if async is enabled for all functions.
  [#9091](https://github.com/bytecodealliance/wasmtime/pull/9091)

* Work has continued on Wasmtime's interpreter backend, Pulley.
  [#9089](https://github.com/bytecodealliance/wasmtime/pull/9089)

* The internal implementation of `input-stream` and `output-stream` for
  filesystems in `wasmtime-wasi` have been refactored to directly implement
  the corresponding host traits. This additionally helps cleanup the internal
  organization of host-side resources in `wasmtime-wasi`.
  [#9129](https://github.com/bytecodealliance/wasmtime/pull/9129)

* Wasmtime now uses the new "user" stack maps in Cranelift rather than the old
  regalloc-based stack maps for GC references.
  [#9082](https://github.com/bytecodealliance/wasmtime/pull/9082)

* Wasmtime's handling of WebAssembly features now works slightly differently
  from before to provide better error messages and fewer panics on unsupported
  WebAssembly features depending on compiler and target selection. Additionally
  the reference-types WebAssembly proposal is always on-by-default regardless of
  crate features.
  [#9158](https://github.com/bytecodealliance/wasmtime/pull/9158)
  [#9162](https://github.com/bytecodealliance/wasmtime/pull/9162)

* The `wasmtime` CLI will now use the async version of I/O where possible to
  properly support `-Wtimeout` and timing out instances blocked in I/O.
  [#9184](https://github.com/bytecodealliance/wasmtime/pull/9184)

### Fixed

* Use `tracing::Instrument` in generated bindings when tracing and async are
  enabled, ensuring that spans aren't present in traces from unrelated async
  tasks.
  [#9217](https://github.com/bytecodealliance/wasmtime/pull/9217)
  [#9263](https://github.com/bytecodealliance/wasmtime/pull/9263)

* Completed support for the `CallHook` API when using the component model.
  [#9196](https://github.com/bytecodealliance/wasmtime/pull/9196)

* The compile time for a component model `enum` type with many cases should be
  much improved now.
  [#9122](https://github.com/bytecodealliance/wasmtime/pull/9122)

* Some minor bugfixes have been made for when Wasmtime is working with split
  DWARF in WebAssembly files.
  [#9109](https://github.com/bytecodealliance/wasmtime/pull/9109)
  [#9132](https://github.com/bytecodealliance/wasmtime/pull/9132)
  [#9134](https://github.com/bytecodealliance/wasmtime/pull/9134)
  [#9139](https://github.com/bytecodealliance/wasmtime/pull/9139)
  [#9151](https://github.com/bytecodealliance/wasmtime/pull/9151)

* An issue with bounds checks and dynamic checks has been fixed in Winch to
  ensure bounds checks are correctly implemented.
  [#9156](https://github.com/bytecodealliance/wasmtime/pull/9156)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
