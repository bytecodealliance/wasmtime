## 47.0.4

Released 2026-08-20

### Fixed

* Guest controlled-size host heap allocation through WASIp3 streams.
  [GHSA-x84v-gj2h-g759](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-x84v-gj2h-g759)

* Filesystem sandbox escape when paths or symlinks contain trailing slashes.
  [GHSA-vqjp-4c8c-hfgg](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-vqjp-4c8c-hfgg)

--------------------------------------------------------------------------------

## 47.0.3

Released 2026-07-31.

### Fixed

* Stores can mix up type indices between engines.
  [GHSA-hgjw-h833-99q9](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-hgjw-h833-99q9)

* Preemption and traps during bulk operations enable breaking internal VM state.
  [GHSA-2hw9-mc66-jc2q](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-2hw9-mc66-jc2q)

--------------------------------------------------------------------------------

## 47.0.2

Released 2026-07-21.

### Fixed

* Fix async-delivered write-closed events for futures.
  [#13914](https://github.com/bytecodealliance/wasmtime/pull/13914)

* Fix call hooks with yields and concurrent execution.
  [#13871](https://github.com/bytecodealliance/wasmtime/pull/13871)

--------------------------------------------------------------------------------

## 47.0.1

Released 2026-07-20.

### Fixed

* Fixed publication of `wasmtime-cli` to crates.io.
  [#13902](https://github.com/bytecodealliance/wasmtime/pull/13902)

--------------------------------------------------------------------------------

## 47.0.0

Released 2026-07-20.

### Added

* Wasmtime now has the WebAssembly GC proposal enabled by default.
  [#13594](https://github.com/bytecodealliance/wasmtime/pull/13594)

* Wasmtime now has the WebAssembly exception-handling proposal enabled by
  default.
  [#13603](https://github.com/bytecodealliance/wasmtime/pull/13603)

* Cranelift now supports the compact unwind format on Mach-O platforms.
  [#13586](https://github.com/bytecodealliance/wasmtime/pull/13586)

* Wasmtime now supports bounds-checked unsafe intrinsics which take spectre
  mitigations into account.
  [#13597](https://github.com/bytecodealliance/wasmtime/pull/13597)

* The `wasmtime::Engine` type now supports reflection to determine the value of
  most `Config` options that were selected during creation.
  [#13671](https://github.com/bytecodealliance/wasmtime/pull/13671)

* Support for fibers on unsupported architectures is now supported through an
  opt-in compile time Cargo feature defining a C API embedders can implement.
  [#13620](https://github.com/bytecodealliance/wasmtime/pull/13620)

* Embedders can now create an `ArrayRef` with a memcpy-style constructor.
  [#13716](https://github.com/bytecodealliance/wasmtime/pull/13716)

* A new `Accessor::poll_ready_for_concurrent_call` API can be used to detect
  when backpressure indicates that a function is ready to be invoked.
  [#13683](https://github.com/bytecodealliance/wasmtime/pull/13683)

### Changed

* Support for wasi-threads and Wasmtime's `wasi-common` crate have been removed.
  For more information see the [associated RFC][rfc] in Wasmtime.
  [#13558](https://github.com/bytecodealliance/wasmtime/pull/13558)

* The `call_indirect` instruction of `final` super types has been optimized when
  the GC proposal is enabled.
  [#13572](https://github.com/bytecodealliance/wasmtime/pull/13572)

* Cranelift's `*_imm` builder helpers have been deprecated in favor of
  `*_imm_{s,u}` helpers.
  [#13583](https://github.com/bytecodealliance/wasmtime/pull/13583)

* The `wasmtime` CLI's syntax supported for `--invoke` with components now
  supports fully qualified WIT interface names.
  [#13564](https://github.com/bytecodealliance/wasmtime/pull/13564)

* Cranelift's `stack_{load,store}` instructions have been removed.
  [#13580](https://github.com/bytecodealliance/wasmtime/pull/13580)

* Cranelift's `{band,bor,bxor}_not` instructions have been removed.
  [#13590](https://github.com/bytecodealliance/wasmtime/pull/13590)

* Cranelift's `global_value` instruction has been removed.
  [#13682](https://github.com/bytecodealliance/wasmtime/pull/13682)

* The `wasmtime wizer` subcommand now supports specifying the initialization
  function with WAVE for components.
  [#13582](https://github.com/bytecodealliance/wasmtime/pull/13582)

* Wasmtime's codegen on aarch64 for the `i32x4.relaxed_dot_i8x16_i7x16_add_s`
  wasm instruction now uses a single `sdot` instruction where possible.
  [#13640](https://github.com/bytecodealliance/wasmtime/pull/13640)

* Wasmtime's `.wasmtime{traps,addrmap}` sections in `*.cwasm` files are now
  significantly smaller.
  [#13628](https://github.com/bytecodealliance/wasmtime/pull/13628)

* Wasmtime now supports removing symbols from `*.cwasm` files and additionally
  has a documentation page about producing minimally-sized `*.cwasm` outputs.
  [#13630](https://github.com/bytecodealliance/wasmtime/pull/13630)

* Wasmtime now suports resources in the `named_imports` option of `bindgen!`.
  [#13666](https://github.com/bytecodealliance/wasmtime/pull/13666)

* Wasmtime now invokes the host's `socket_addr_check` callback for implicit
  binds performed in WASIp3.
  [#13677](https://github.com/bytecodealliance/wasmtime/pull/13677)

* Wasmtime's component-to-component sync-to-sync fused adapters have been
  optimized.
  [#13695](https://github.com/bytecodealliance/wasmtime/pull/13695)

* Cranelift now supports RISC-V's Zvbb extension.
  [#13738](https://github.com/bytecodealliance/wasmtime/pull/13738)

[rfc]: https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-remove-wasi-threads.md

### Fixed

* The `IPV6_V6ONLY` option is now set for UDP sockets.
  [#13596](https://github.com/bytecodealliance/wasmtime/pull/13596)

* Uncaught wasm exceptions at component boundaries are now turned into traps to
  prevent components catching exceptions from other components.
  [#13613](https://github.com/bytecodealliance/wasmtime/pull/13613)

* Addresses being sent do with WASIp3 and UDP are now validated as they are in
  WASIp2.
  [#13631](https://github.com/bytecodealliance/wasmtime/pull/13631)

* GC roots on component model fiber stacks are now properly traced.
  [#13693](https://github.com/bytecodealliance/wasmtime/pull/13693)

* An unaligned load when reading debug state slot values has been fixed.
  [#13791](https://github.com/bytecodealliance/wasmtime/pull/13791)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
* [46.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-46.0.0/RELEASES.md)
* [45.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-45.0.0/RELEASES.md)
* [44.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-44.0.0/RELEASES.md)
* [43.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-43.0.0/RELEASES.md)
* [42.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-42.0.0/RELEASES.md)
* [41.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-41.0.0/RELEASES.md)
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
