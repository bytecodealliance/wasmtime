## 29.0.0

Released 2025-01-20.

### Added

* Winch now supports epoch-based interruption.
  [#9737](https://github.com/bytecodealliance/wasmtime/pull/9737)

* Pulley, Wasmtime's WebAssembly interpreter, has seen quite a lot of progress
  and support fleshed out. It's still not 100% complete but should be about
  ready to start kicking the tires.
  [#9744](https://github.com/bytecodealliance/wasmtime/pull/9744)

* The Wasmtime CLI now supports a `-Wextended-const` flag to control whether the
  `extended-const` wasm proposal is enabled or not.
  [#9768](https://github.com/bytecodealliance/wasmtime/pull/9768)

* Work continues to progress on the AArch64 Winch backend, bringing it closer to
  completion.
  [#9762](https://github.com/bytecodealliance/wasmtime/pull/9762)
  [#9767](https://github.com/bytecodealliance/wasmtime/pull/9767)
  [#9751](https://github.com/bytecodealliance/wasmtime/pull/9751)
  [#9784](https://github.com/bytecodealliance/wasmtime/pull/9784)
  [#9781](https://github.com/bytecodealliance/wasmtime/pull/9781)
  [#9792](https://github.com/bytecodealliance/wasmtime/pull/9792)
  [#9787](https://github.com/bytecodealliance/wasmtime/pull/9787)
  [#9798](https://github.com/bytecodealliance/wasmtime/pull/9798)
  [#9850](https://github.com/bytecodealliance/wasmtime/pull/9850)

* Wasmtime now supports a "custom code publisher" which can be useful when
  Wasmtime doesn't have built-in support for a particular environment.
  [#9778](https://github.com/bytecodealliance/wasmtime/pull/9778)

* Configuration options have been added for `wasmtime-wasi-http` outgoing
  bodies.
  [#9800](https://github.com/bytecodealliance/wasmtime/pull/9800)

* Log prefixes can now be disabled for the `wasmtime serve` command.
  [#9821](https://github.com/bytecodealliance/wasmtime/pull/9821)

* A new `WASMTIME_LOG_NO_CONTEXT` environment variable was added to live
  alongside `WASMTIME_LOG`.
  [#9902](https://github.com/bytecodealliance/wasmtime/pull/9902)

* Release artifacts for aarch64-musl targets are now available.
  [#9934](https://github.com/bytecodealliance/wasmtime/pull/9934)

### Changed

* Wasmtime libcalls now return whether a trap happened rather than raising a
  trap directly to better prepare for the Pulley interpreter and an eventual
  implementation of Wasm exception-handling.
  [#9710](https://github.com/bytecodealliance/wasmtime/pull/9710)

* Wasmtime will now use the Pulley interpreter by default on platforms that
  are not supported by Cranelift.
  [#9741](https://github.com/bytecodealliance/wasmtime/pull/9741)

* Demangling symbols in profiling and debugging has improved to handle failures
  to demangle C++ symbols.
  [#9756](https://github.com/bytecodealliance/wasmtime/pull/9756)

* WASI WIT files have been updated to 0.2.3.
  [#9807](https://github.com/bytecodealliance/wasmtime/pull/9807)

* Wasmtime's `bindgen!` macro in `async` mode no longer uses `#[async_trait]`
  an instead natively uses `async fn` in traits.
  [#9867](https://github.com/bytecodealliance/wasmtime/pull/9867)

* Floats are no longer canonicalized flowing into or out of components.
  [#9879](https://github.com/bytecodealliance/wasmtime/pull/9879)

* Instance methods are now translated to static methods in DWARF translation.
  [#9898](https://github.com/bytecodealliance/wasmtime/pull/9898)

* The C API now supports debug builtins for debugging guest code.
  [#9915](https://github.com/bytecodealliance/wasmtime/pull/9915)

### Fixed

* The header file for `wasmtime_instance_pre_instantiate` in the C API has been
  fixed.
  [#9770](https://github.com/bytecodealliance/wasmtime/pull/9770)

* WebAssembly DWARF is more conservative in its GC pass during translation to
  native DWARF.
  [#9829](https://github.com/bytecodealliance/wasmtime/pull/9829)

* Debugging intrinsics are fixed on Linux to be exported now.
  [#9866](https://github.com/bytecodealliance/wasmtime/pull/9866)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
