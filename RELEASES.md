## 41.0.2

Released 2026-02-03.

### Fixed

* Reduce the dependencies of the `wasmtime-internal-jit-icache-coherence` crate.
  [#12446](https://github.com/bytecodealliance/wasmtime/pull/12446)

--------------------------------------------------------------------------------

## 41.0.1

Released 2026-01-26.

### Fixed

* Fixed a bug in lowering of `f64.copysign` on x86-64 whereby when combined
  with an `f64.load`, the resulting machine code could read 16 bytes rather
  than 8 bytes. This could result in a segfault when Wasmtime is configured
  without signals-based traps.

--------------------------------------------------------------------------------

## 41.0.0

Released 2026-01-20.

### Added

* Support for `{Future,Stream}Any` in the component model has improved.
  [#12142](https://github.com/bytecodealliance/wasmtime/pull/12142)

* Wasmtime has initial support for breakpoints and single-stepping with the
  `debug` feature for guest programs.
  [#12133](https://github.com/bytecodealliance/wasmtime/pull/12133)

* Wasmtime has begun adding a new `Error` type which is similar to
  `anyhow::Error` but supports gracefully handling OOM. Wasmtime still uses
  `anyhow::Error` but this will change in the future to `wasmtime::Error` which
  will be a distinct type.
  [#12163](https://github.com/bytecodealliance/wasmtime/pull/12163)

* An initial top-level crate for async-debugging guest programs has been added.
  [#12183](https://github.com/bytecodealliance/wasmtime/pull/12183)

### Changed

* Cranelift now optimizes redundant `select` + `icmp` instructions.
  [#12135](https://github.com/bytecodealliance/wasmtime/pull/12135)

* Synchronous component model functions can no longer block before returning.
  This implements a change in the upstream specification to the upcoming `async`
  support in the component model which places stricter restrictions on
  non-`async` functions and their ability to perform blocking operations.
  [#12043](https://github.com/bytecodealliance/wasmtime/pull/12043)

* Frame iteration in `debug` mode now visits all activations which enables
  seeing all frames from recursive wasm calls.
  [#12176](https://github.com/bytecodealliance/wasmtime/pull/12176)

* Wasmtime now requires Rust 1.90.0 or later.
  [#12167](https://github.com/bytecodealliance/wasmtime/pull/12167)

* Intra-component stream/future reads/writes are now allowed for simple data
  types.
  [#12181](https://github.com/bytecodealliance/wasmtime/pull/12181)

* The `POLL` callback code has been removed from the canonical ABI for async
  functions and the `waitable-set.poll` function no longer yields.
  [#12182](https://github.com/bytecodealliance/wasmtime/pull/12182)

* Guest-to-guest adapters injected by Wasmtime now have improved trapping error
  messages.
  [#12215](https://github.com/bytecodealliance/wasmtime/pull/12215)

### Fixed

* `#[derive(Lift)]` for enums with exactly 256 cases has been fixed.
  [#12140](https://github.com/bytecodealliance/wasmtime/pull/12140)

* With component-model-async support recursively calling a guest from a host
  function has now been fixed.
  [#12152](https://github.com/bytecodealliance/wasmtime/pull/12152)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
