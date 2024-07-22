## 23.0.0

Released 2024-07-22.

### Added

* Support for DWARF debugging information with native debuggers is now
  implemented for components.
  [#8693](https://github.com/bytecodealliance/wasmtime/pull/8693)

* CLIF frontends can now define their own stack maps.
  [#8728](https://github.com/bytecodealliance/wasmtime/pull/8728)
  [#8876](https://github.com/bytecodealliance/wasmtime/pull/8876)

* Wasmtime now supports the custom-page-sizes proposal.
  [#8763](https://github.com/bytecodealliance/wasmtime/pull/8763)

* This project now publishes a crate named
  `wasi-preview1-component-adapter-provider` which provides the WASIp1 adapters
  as constants in Rust code.
  [#8874](https://github.com/bytecodealliance/wasmtime/pull/8874)

### Changed

* Call hooks now have access to the full `StoreContextMut<T>`.
  [#8791](https://github.com/bytecodealliance/wasmtime/pull/8791)

* Call hooks have been moved behind an off-by-default compile-time Cargo feature
  named `call-hook`.
  [#8795](https://github.com/bytecodealliance/wasmtime/pull/8795)
  [#8808](https://github.com/bytecodealliance/wasmtime/pull/8808)

* Wasmtime's minimum supported Rust version is now 1.77.0.
  [#8796](https://github.com/bytecodealliance/wasmtime/pull/8796)

* Resumable traps have been removed from Cranelift.
  [#8809](https://github.com/bytecodealliance/wasmtime/pull/8809)

* Traps are not GC safepoints any more in Cranelift.
  [#8810](https://github.com/bytecodealliance/wasmtime/pull/8810)

* Support for Intel memory protection keys is now disabled by default at compile
  time and is gated behind a Cargo feature.
  [#8813](https://github.com/bytecodealliance/wasmtime/pull/8813)

* Exports from components have been refactored and redesigned to support
  skipping name lookups at runtime where possible.
  [#8786](https://github.com/bytecodealliance/wasmtime/pull/8786)

* Wasmtime's lookup of versioned component exports now takes semver into
  account in the same manner as imports.
  [#8830](https://github.com/bytecodealliance/wasmtime/pull/8830)

* Wasmtime's guest profiler will now take samples at hostcall boundaries.
  [#8802](https://github.com/bytecodealliance/wasmtime/pull/8802)

* Wasmtime's pooling allocator now by default allows 32-bit linear memories to
  grow to their full size of 4G.
  [#8849](https://github.com/bytecodealliance/wasmtime/pull/8849)

* The size of WASI adapter binaries has been optimized.
  [#8858](https://github.com/bytecodealliance/wasmtime/pull/8858)
  [#8859](https://github.com/bytecodealliance/wasmtime/pull/8859)

* The `wasmtime-wasi-http` crate has been refactored to better match the
  `wasmtime-wasi` crate.
  [#8861](https://github.com/bytecodealliance/wasmtime/pull/8861)

* Support for caching `call_indirect` sites has been removed.
  [#8881](https://github.com/bytecodealliance/wasmtime/pull/8881)

* Wasmtime's x86\_64 binary releases are now based on AlmaLinux 8 instead of
  CentOS 7.
  [#8892](https://github.com/bytecodealliance/wasmtime/pull/8892)

### Fixed

* An issue with generated `.debug_loc` sections for native debuggers has been
  fixed.
  [#8753](https://github.com/bytecodealliance/wasmtime/pull/8753)

* Wasmtime's `no_std` build for riscv64 has been fixed.
  [#8770](https://github.com/bytecodealliance/wasmtime/pull/8770)

* A bug related to lost `Waker` instances with async stdio streams has been
  fixed.
  [#8782](https://github.com/bytecodealliance/wasmtime/pull/8782)

* Configuration of `trappable_error_type` has been improved in Wasmtime's
  `bindgen!` macro.
  [#8833](https://github.com/bytecodealliance/wasmtime/pull/8833)

* Prints to stdout/stderr without a newline now work better with `wasmtime
  serve`.
  [#8877](https://github.com/bytecodealliance/wasmtime/pull/8877)

* An issue with `br_if` and stack-related state has been fixed in Winch.
  [#8886](https://github.com/bytecodealliance/wasmtime/pull/8886)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
