## 37.0.1

Released 2025-09-23.

### Fixed

* Cranelift's `cranelift-jit` crate now properly applies relocations to `ADRP`
  instructions on aarch64; a zero-extension on the offset was fixed to properly
  sign-extend instead.
  [#11734](https://github.com/bytecodealliance/wasmtime/pull/11734)

## 37.0.0

Released 2025-09-20.

### Added

* Wasmtime now fully implements the WebAssembly exception-handling proposal.
  Support is still disabled by default but is ready for testing. The proposal
  will be enabled by default in a future release of Wasmtime.
  [#11326](https://github.com/bytecodealliance/wasmtime/pull/11326)

* An initial implementation of WASIp3 is available for the `0.3.0-rc-2025-08-15`
  tag made for the WASIp3 release. Note that this is not production ready yet
  but is an excellent time to start kicking the tires in preparation for an
  upcoming officialy WASIp3 0.3.0 release. Users of the CLI can opt-in with
  `-Sp3 -Wcomponent-model-async`.
  [#11406](https://github.com/bytecodealliance/wasmtime/pull/11406)
  [#11423](https://github.com/bytecodealliance/wasmtime/pull/11423)
  [#11443](https://github.com/bytecodealliance/wasmtime/pull/11443)

* Wasmtime has initial support for the Linux `PAGEMAP_SCAN` ioctl which can
  greatly improve instantiation throughput in scenarios with a high number of
  instantiations and short instance lifetime. This support is disabled by
  default but will likely be enabled by default in a future release.
  [#11372](https://github.com/bytecodealliance/wasmtime/pull/11372)
  [#11433](https://github.com/bytecodealliance/wasmtime/pull/11433)

* GC support can now be configured in `Config` and not only through crate
  features through `Config::gc_support`.
  [#11463](https://github.com/bytecodealliance/wasmtime/pull/11463)

* Wasmtime now supports reading metrics of the pooling allocator at runtime.
  [#11490](https://github.com/bytecodealliance/wasmtime/pull/11490)

* The `ManuallyRooted` type is now replaced with `OwnedRooted` which is intended
  to make management of GC object lifetimes on the host easier.
  [#11514](https://github.com/bytecodealliance/wasmtime/pull/11514)

* Wasmtime's documentation of the C++ embedding API and examples has been
  expanded.
  [#11569](https://github.com/bytecodealliance/wasmtime/pull/11569)

* Wasmtime's support for the stack-switching WebAssembly proposal continues to
  progress on x86\_64 Linux.
  [#11003](https://github.com/bytecodealliance/wasmtime/pull/11003)

### Changed

* The `preview0` and `preview1` modules and features in the `wasmtime-wasi`
  crate are now called `p0` and `p1`.
  [#11380](https://github.com/bytecodealliance/wasmtime/pull/11380)

* Release artifacts for the C API are now unconditionally built with unwind
  tables.
  [#11383](https://github.com/bytecodealliance/wasmtime/pull/11383)

* Wasmtime now requires Rust 1.87.0 or later to build.
  [#11396](https://github.com/bytecodealliance/wasmtime/pull/11396)

* The component-model-async gated `AbortHandle` is now named `JoinHandle`.
  [#11414](https://github.com/bytecodealliance/wasmtime/pull/11414)

* Wasmtime's internal implementation details are now `async` in many more
  locations to help ensure the implementation is more sound.
  [#11411](https://github.com/bytecodealliance/wasmtime/pull/11411)
  [#11416](https://github.com/bytecodealliance/wasmtime/pull/11416)
  [#11442](https://github.com/bytecodealliance/wasmtime/pull/11442)
  [#11444](https://github.com/bytecodealliance/wasmtime/pull/11444)
  [#11457](https://github.com/bytecodealliance/wasmtime/pull/11457)
  [#11460](https://github.com/bytecodealliance/wasmtime/pull/11460)
  [#11461](https://github.com/bytecodealliance/wasmtime/pull/11461)
  [#11468](https://github.com/bytecodealliance/wasmtime/pull/11468)
  [#11470](https://github.com/bytecodealliance/wasmtime/pull/11470)
  [#11481](https://github.com/bytecodealliance/wasmtime/pull/11481)
  [#11496](https://github.com/bytecodealliance/wasmtime/pull/11496)

* Component-model-async primitives such as streams, tasks, etc, now use the same
  table as resources in a component. This means that guest-visible allocated
  indices are updated slightly.
  [#11374](https://github.com/bytecodealliance/wasmtime/pull/11374)

* Wasmtime's precompiled binaries available from CI now include the
  `component-model-async` feature.
  [#11429](https://github.com/bytecodealliance/wasmtime/pull/11429)

* C API release artifacts are now built with LTO so they have a smaller size.
  [#11483](https://github.com/bytecodealliance/wasmtime/pull/11483)

* Code can no longer be loaded on `x86_64-unknown-none` by default without
  opting-in to a contract that either the host is compiled with SSE2 support or
  wasm is compiled with enough features that libcalls aren't used.
  [#11553](https://github.com/bytecodealliance/wasmtime/pull/11553)

* Host support for component model async futures/streams has been updated to a
  new API.
  [#11515](https://github.com/bytecodealliance/wasmtime/pull/11515)

### Fixed

* GC of dead DWARF has been improved.
  [#11402](https://github.com/bytecodealliance/wasmtime/pull/11402)

* Wasm-gc branching instructions now correctly check for fuel.
  [#11426](https://github.com/bytecodealliance/wasmtime/pull/11426)

* The `array.new_default` instruction now checks for fuel/epochs in its inner
  loop.
  [#11428](https://github.com/bytecodealliance/wasmtime/pull/11428)

* The "min" C API artifacts now have correct headers.
  [#11479](https://github.com/bytecodealliance/wasmtime/pull/11479)

* GC OOM during const eval no longer panics.
  [#11557](https://github.com/bytecodealliance/wasmtime/pull/11557)

* Wasmtime now properly respects a disabled `std` feature even on targets which
  have `std` available.
  [#11568](https://github.com/bytecodealliance/wasmtime/pull/11568)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
