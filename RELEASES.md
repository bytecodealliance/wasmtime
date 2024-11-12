## 27.0.0

Unreleased.

### Added

* Support for the Wasm GC proposal is now complete. A new "null" GC has been
  also added which does not ever collect garbage.
  [#9389](https://github.com/bytecodealliance/wasmtime/pull/9389)
  [#9392](https://github.com/bytecodealliance/wasmtime/pull/9392)
  [#9401](https://github.com/bytecodealliance/wasmtime/pull/9401)
  [#9435](https://github.com/bytecodealliance/wasmtime/pull/9435)
  [#9437](https://github.com/bytecodealliance/wasmtime/pull/9437)
  [#9438](https://github.com/bytecodealliance/wasmtime/pull/9438)
  [#9446](https://github.com/bytecodealliance/wasmtime/pull/9446)
  [#9448](https://github.com/bytecodealliance/wasmtime/pull/9448)
  [#9454](https://github.com/bytecodealliance/wasmtime/pull/9454)
  [#9455](https://github.com/bytecodealliance/wasmtime/pull/9455)
  [#9484](https://github.com/bytecodealliance/wasmtime/pull/9484)

* Unstable WIT APIs now have feature gates configured at link-time and new
  `-Scli-exit-with-code` / `-Snetwork-error-code` options are available as well.
  [#9381](https://github.com/bytecodealliance/wasmtime/pull/9381)
  [#9276](https://github.com/bytecodealliance/wasmtime/pull/9276)

* Initial support for the wide-arithmetic proposal has been implemented.
  [#9403](https://github.com/bytecodealliance/wasmtime/pull/9403)
  [#9500](https://github.com/bytecodealliance/wasmtime/pull/9500)

* Guests on s390x now implement the "inline probestacks" for Cranelift to more
  robustly detect stack overflows.
  [#9423](https://github.com/bytecodealliance/wasmtime/pull/9423)

* Missing CLI options for the pooling allocator have been filled out.
  [#9447](https://github.com/bytecodealliance/wasmtime/pull/9447)

* Cranelift now supports 128-bit atomics on x64.
  [#9459](https://github.com/bytecodealliance/wasmtime/pull/9459)

* A new Cargo feature has been added to the `wasmtime` crate to reexport the
  `wasmparser` dependency.
  [#9485](https://github.com/bytecodealliance/wasmtime/pull/9485)

* Support for a new PyTorch backend for wasi-nn has been added.
  [#9234](https://github.com/bytecodealliance/wasmtime/pull/9234)

* A new `-Cnative-unwind-info` flag has been added to the `wasmtime` CLI.
  [#9494](https://github.com/bytecodealliance/wasmtime/pull/9494)

* Initial support for illumos has been added.
  [#9535](https://github.com/bytecodealliance/wasmtime/pull/9535)

* A new `Caller::get_module_export` API has been added.
  [#9525](https://github.com/bytecodealliance/wasmtime/pull/9525)

* Basic debug logging has been added to the debug info transformatino.
  [#9526](https://github.com/bytecodealliance/wasmtime/pull/9526)

### Changed

* The WASI WITs vendored are now updated to 0.2.2.
  [#9395](https://github.com/bytecodealliance/wasmtime/pull/9395)

* The `wasmtime-wasi-runtime-config` is now named `wasmtime-wasi-config`.
  [#9404](https://github.com/bytecodealliance/wasmtime/pull/9404)

* Documentation on the implementation status of WebAssembly proposals has been
  updated.
  [#9434](https://github.com/bytecodealliance/wasmtime/pull/9434)

* Wasmtime's WASI documentation has been overhauled.
  [#9471](https://github.com/bytecodealliance/wasmtime/pull/9471)

* The `wasi_config_preopen_dir` in Wasmtime's C API now takes file/directory
  permissions.
  [#9477](https://github.com/bytecodealliance/wasmtime/pull/9477)

* Detection of libunwind vs libgcc is now done with weak symbols.
  [#9479](https://github.com/bytecodealliance/wasmtime/pull/9479)

* Winch has improved detection of unsupported features in a `Config`.
  [#9490](https://github.com/bytecodealliance/wasmtime/pull/9490)

* Winch now supports fuel-based interruption.
  [#9472](https://github.com/bytecodealliance/wasmtime/pull/9472)

* Wasmtime's minimum supported Rust version is now 1.80.
  [#9496](https://github.com/bytecodealliance/wasmtime/pull/9496)

* ISLE no longer supports scheme-style booleans.
  [#9522](https://github.com/bytecodealliance/wasmtime/pull/9522)

* ISLE now supports block comments.
  [#9529](https://github.com/bytecodealliance/wasmtime/pull/9529)

* Support for shared memory in the C API has been added.
  [#9507](https://github.com/bytecodealliance/wasmtime/pull/9507)

* Configuration options for guard size regions have been merged into a single
  option.
  [#9528](https://github.com/bytecodealliance/wasmtime/pull/9528)

### Fixed

* Double-registration of debug information for modules in components has been
  fixed.
  [#9470](https://github.com/bytecodealliance/wasmtime/pull/9470)

* A panic on AArch64 for vector constants has been fixed.
  [#9482](https://github.com/bytecodealliance/wasmtime/pull/9482)

* A miscompile with `sdiv` and `INT_MIN / -1` has been fixed on aarch64.
  [#9541](https://github.com/bytecodealliance/wasmtime/pull/9541)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
