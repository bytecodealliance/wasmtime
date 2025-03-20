## 31.0.0

Released 2025-03-20.

### Added

* Winch's implementation of the SIMD proposal for WebAssembly is now
  feature-complete (but still being fuzzed).
  [#10180](https://github.com/bytecodealliance/wasmtime/pull/10180)
  [#10170](https://github.com/bytecodealliance/wasmtime/pull/10170)
  [#10203](https://github.com/bytecodealliance/wasmtime/pull/10203)
  [#10202](https://github.com/bytecodealliance/wasmtime/pull/10202)
  [#10210](https://github.com/bytecodealliance/wasmtime/pull/10210)
  [#10213](https://github.com/bytecodealliance/wasmtime/pull/10213)
  [#10224](https://github.com/bytecodealliance/wasmtime/pull/10224)
  [#10205](https://github.com/bytecodealliance/wasmtime/pull/10205)
  [#10226](https://github.com/bytecodealliance/wasmtime/pull/10226)
  [#10228](https://github.com/bytecodealliance/wasmtime/pull/10228)
  [#10236](https://github.com/bytecodealliance/wasmtime/pull/10236)
  [#10241](https://github.com/bytecodealliance/wasmtime/pull/10241)
  [#10243](https://github.com/bytecodealliance/wasmtime/pull/10243)
  [#10247](https://github.com/bytecodealliance/wasmtime/pull/10247)
  [#10271](https://github.com/bytecodealliance/wasmtime/pull/10271)
  [#10284](https://github.com/bytecodealliance/wasmtime/pull/10284)
  [#10288](https://github.com/bytecodealliance/wasmtime/pull/10288)
  [#10296](https://github.com/bytecodealliance/wasmtime/pull/10296)

* The pytorch implementation in wasmtime-wasi-nn now has GPU support.
  [#10204](https://github.com/bytecodealliance/wasmtime/pull/10204)

* Cranelift now supports emitting the AArch64 `extr` instruction.
  [#10229](https://github.com/bytecodealliance/wasmtime/pull/10229)

* Cranelift now supports emitting the x64 `shld` instruction.
  [#10233](https://github.com/bytecodealliance/wasmtime/pull/10233)

* Initial support for the stack-switching proposal has started to land, but it
  is not complete just yet.
  [#10251](https://github.com/bytecodealliance/wasmtime/pull/10251)
  [#10265](https://github.com/bytecodealliance/wasmtime/pull/10265)
  [#10255](https://github.com/bytecodealliance/wasmtime/pull/10255)

### Changed

* Pulley's implementation of loads/stores to linear memory has changed to
  better support optimizations and reduction of interpreter opcodes in the
  final binary.
  [#10154](https://github.com/bytecodealliance/wasmtime/pull/10154)

* Cranelift's verifier now ensures that integers used as address types have the
  correct width.
  [#10209](https://github.com/bytecodealliance/wasmtime/pull/10209)

* Wasmtime and Cranelift's minimum supported version of Rust is now 1.83.0.
  [#10264](https://github.com/bytecodealliance/wasmtime/pull/10264)

* Wasmtime now mentions the filename when the input cannot be opened on the CLI.
  [#10292](https://github.com/bytecodealliance/wasmtime/pull/10292)

* All types are now generated in `component::bindgen!`, even if they're not
  reachable.
  [#10311](https://github.com/bytecodealliance/wasmtime/pull/10311)

* Tables allocated with the system allocator now use `alloc_zeroed` (aka
  `calloc`) for allocation.
  [#10313](https://github.com/bytecodealliance/wasmtime/pull/10313)

### Fixed

* GC: the is-null-or-i31ref checks have been fixed.
  [#10221](https://github.com/bytecodealliance/wasmtime/pull/10221)

* GC: an incorrect assertion and canonicalized types for runtime usage has been
  fixed.
  [#10223](https://github.com/bytecodealliance/wasmtime/pull/10223)

* GC: subtype checks for imported globals during instantiation have been fixed.
  [#10304](https://github.com/bytecodealliance/wasmtime/pull/10304)

* GC: exposing references to wasm in the `gc_alloc_raw` libcall has been fixed.
  [#10322](https://github.com/bytecodealliance/wasmtime/pull/10322)

* Winch's fuel checks correctly sync fuel before the check now.
  [#10231](https://github.com/bytecodealliance/wasmtime/pull/10231)

* Winch's treatment of stores and other trapping ops has been fixed on AArch64.
  [#10201](https://github.com/bytecodealliance/wasmtime/pull/10201)

* Winch's handling of the shadow stack pointer has been fixed on AArch64.
  [#10263](https://github.com/bytecodealliance/wasmtime/pull/10263)

* Winch's handling of address calculations has been fixed on AArch64.
  [#10297](https://github.com/bytecodealliance/wasmtime/pull/10297)

* Winch's handling of multivalue return of constants has ben fixed.
  [#10315](https://github.com/bytecodealliance/wasmtime/pull/10315)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
