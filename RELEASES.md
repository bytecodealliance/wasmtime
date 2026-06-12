## 46.0.0

Unreleased.

### Added

* Added opt-in support for the WebAssembly branch-hinting proposal: the
  `metadata.code.branch_hint` custom section is parsed and used to mark cold
  blocks during Cranelift compilation, behind `Config::wasm_branch_hinting`
  (off by default).
  [#13459](https://github.com/bytecodealliance/wasmtime/pull/13459)

* Wasmtime's C API now supports type reflection of GC values.
  [#13268](https://github.com/bytecodealliance/wasmtime/pull/13268)

* Wasmtime now supports being compiled to `arm64_32` platforms.
  [#13259](https://github.com/bytecodealliance/wasmtime/pull/13259)

* The `cranelift-frontend` crate,the RISC-V Cranelift backend, and Wasmtime's
  `component-model-async` feature now support `no_std`.
  [#13401](https://github.com/bytecodealliance/wasmtime/pull/13401)
  [#13479](https://github.com/bytecodealliance/wasmtime/pull/13479)
  [#13533](https://github.com/bytecodealliance/wasmtime/pull/13533)

* Wasmtime has initial support for the component model `(implements "...")`
  feature.
  [#13361](https://github.com/bytecodealliance/wasmtime/pull/13361)
  [#13497](https://github.com/bytecodealliance/wasmtime/pull/13497)
  [#13513](https://github.com/bytecodealliance/wasmtime/pull/13513)

* The `wasmtime` CLI now supports configuring the `initial-cwd` property of
  WASI.
  [#13468](https://github.com/bytecodealliance/wasmtime/pull/13468)

* The `wasmtime serve` subcommand now supports `--header` to set headers for the
  guest.
  [#13471](https://github.com/bytecodealliance/wasmtime/pull/13471)

* Wasmtime supports WASI 0.2.12, which notably includes `exit-with-code` in
  stable.
  [#13536](https://github.com/bytecodealliance/wasmtime/pull/13536)

* Wasmtime's `component-model-async-bytes` feature was renamed to
  `component-model-bytes` and now lifting/lowering using `Bytes` and `BytesMut`
  is directly supported.
  [#13366](https://github.com/bytecodealliance/wasmtime/pull/13366)

* Wasmtime now exposes the async call stack of components through its public API
  which can be used to determine which root task performed an import call.
  [#13510](https://github.com/bytecodealliance/wasmtime/pull/13510)

* Wasmtime now supports WASI 0.3.0 by default and the `component-model-async`
  wasm feature is now enabled by default.
  [#13612](https://github.com/bytecodealliance/wasmtime/pull/13612)


### Changed

* Performance of bulk-data-transfer instructions such as
  `{array,table,memory}.{copy,fill,init}` have been improved.
  [#13312](https://github.com/bytecodealliance/wasmtime/pull/13312)
  [#13367](https://github.com/bytecodealliance/wasmtime/pull/13367)
  [#13368](https://github.com/bytecodealliance/wasmtime/pull/13368)
  [#13382](https://github.com/bytecodealliance/wasmtime/pull/13382)
  [#13407](https://github.com/bytecodealliance/wasmtime/pull/13407)
  [#13424](https://github.com/bytecodealliance/wasmtime/pull/13424)
  [#13438](https://github.com/bytecodealliance/wasmtime/pull/13438)
  [#13460](https://github.com/bytecodealliance/wasmtime/pull/13460)
  [#13524](https://github.com/bytecodealliance/wasmtime/pull/13524)

* Codegen for conditions based on `ctz` or `clz` has been optimized.
  [#13332](https://github.com/bytecodealliance/wasmtime/pull/13332)
  [#13343](https://github.com/bytecodealliance/wasmtime/pull/13343)

* Cranelift now optimizes conditional branches of constant conditions into
  unconditional jumps.
  [#13267](https://github.com/bytecodealliance/wasmtime/pull/13267)
  [#13391](https://github.com/bytecodealliance/wasmtime/pull/13391)

* Wasmtime's copying collector now has an in-wasm fast path for its bump
  allocator.
  [#13323](https://github.com/bytecodealliance/wasmtime/pull/13323)

* Wasmtime's GC implementation has been hardened in the face of GC heap
  corruption to avoid panicking or aborting. Corruption is returned as a
  `WasmtimeBug` type for embedders to detect and safely tear down the
  store/instance.
  [#13321](https://github.com/bytecodealliance/wasmtime/pull/13321)
  [#13320](https://github.com/bytecodealliance/wasmtime/pull/13320)

* Cranelift's `MemFlags` type is now renamed to `MemFlagsData`, and
  `AliasRegion`s are now stored in the DFG.
  [#13353](https://github.com/bytecodealliance/wasmtime/pull/13353)
  [#13354](https://github.com/bytecodealliance/wasmtime/pull/13354)

* Wasmtime now handles OOM gracefully in more situations.
  [#13371](https://github.com/bytecodealliance/wasmtime/pull/13371)
  [#13372](https://github.com/bytecodealliance/wasmtime/pull/13372)
  [#13374](https://github.com/bytecodealliance/wasmtime/pull/13374)
  [#13375](https://github.com/bytecodealliance/wasmtime/pull/13375)
  [#13376](https://github.com/bytecodealliance/wasmtime/pull/13376)
  [#13377](https://github.com/bytecodealliance/wasmtime/pull/13377)
  [#13378](https://github.com/bytecodealliance/wasmtime/pull/13378)
  [#13379](https://github.com/bytecodealliance/wasmtime/pull/13379)
  [#13388](https://github.com/bytecodealliance/wasmtime/pull/13388)
  [#13413](https://github.com/bytecodealliance/wasmtime/pull/13413)
  [#13412](https://github.com/bytecodealliance/wasmtime/pull/13412)
  [#13414](https://github.com/bytecodealliance/wasmtime/pull/13414)

* Cranelift's egraph rewrite pass now uses a concept of fuel to avoid
  exponential blowup of rewrites.
  [#13390](https://github.com/bytecodealliance/wasmtime/pull/13390)

* Wasmtime now consumes fuel in bulk-data-transfer instructions proportional to
  the size of the transfer.
  [#13393](https://github.com/bytecodealliance/wasmtime/pull/13393)
  [#13448](https://github.com/bytecodealliance/wasmtime/pull/13448)

* Cranelift's ISLE format now supports `struct`s as well as tuple fields for
  structs/enums.
  [#13319](https://github.com/bytecodealliance/wasmtime/pull/13319)
  [#13335](https://github.com/bytecodealliance/wasmtime/pull/13335)

* Wasmtime's implementation of passive element and data segments is now
  optimized to perform more work in wasm itself and has a refactored
  representation on the host.
  [#13394](https://github.com/bytecodealliance/wasmtime/pull/13394)
  [#13444](https://github.com/bytecodealliance/wasmtime/pull/13444)

* Heuristics for triggering GC in the DRC collector have been adjusted to avoid
  blowups seen in the wild.
  [#13422](https://github.com/bytecodealliance/wasmtime/pull/13422)

* Wasmtime's default garbage collector is now the copying collector instead of
  the deferred-reference-counting collector. This collector should be more
  performant in most situations and additionally have the ability to collect
  cycles.
  [#13439](https://github.com/bytecodealliance/wasmtime/pull/13439)

* Wasmtime now traps if a waitable is being waited on synchronously and
  additionally added to a `waitable-set`.
  [#13415](https://github.com/bytecodealliance/wasmtime/pull/13415)

* Wasmtime's behavior with `subtask.cancel` is now adjusted to resume the
  cancelled task immediately instead of always yielding.
  [#13443](https://github.com/bytecodealliance/wasmtime/pull/13443)

* The `wasmtime_wasi_http::handler` module has had its interface overhauled to
  better handle configuring the lifecycle of a request as it flows through
  the system in terms of timeouts and such.
  [#13404](https://github.com/bytecodealliance/wasmtime/pull/13404)

* Most of Wasmtime's instance initialization is now compiled into a per-module
  initialization function rather than happening through the host in Wasmtime.
  [#13487](https://github.com/bytecodealliance/wasmtime/pull/13487)

* The `InstanceExportLookup` trait has been generalized into `ExportLookup`, and
  this is now optionally implemented for `wit_parser::ItemName` with the
  `wit-parser` crate feature.
  [#13505](https://github.com/bytecodealliance/wasmtime/pull/13505)

* Many of Cranelift's `*_imm` instructions have been removed in favor as they
  were just sugar over other opcodes. Builder-style methods remain, however.
  [#13527](https://github.com/bytecodealliance/wasmtime/pull/13527)
  [#13541](https://github.com/bytecodealliance/wasmtime/pull/13541)
  [#13543](https://github.com/bytecodealliance/wasmtime/pull/13543)
  [#13545](https://github.com/bytecodealliance/wasmtime/pull/13545)
  [#13548](https://github.com/bytecodealliance/wasmtime/pull/13548)
  [#13553](https://github.com/bytecodealliance/wasmtime/pull/13553)

* Wasmtime's caching behavior is no longer gated on `cfg(debug_assertions)` and
  has been adjusted to handle a git source differently.
  [#13535](https://github.com/bytecodealliance/wasmtime/pull/13535)

* Bindings generation for store-using `*WithStore` traits now have a type
  parameter of the store on the trait itself instead of on every method.
  [#13549](https://github.com/bytecodealliance/wasmtime/pull/13549)

* Wasmtime now requires Rust 1.94.0 to compile.
  [#13547](https://github.com/bytecodealliance/wasmtime/pull/13547)

### Fixed

* Wasmtime's copying collector has had a few bugs related to how it's translated
  to CLIF fixed.
  [#13381](https://github.com/bytecodealliance/wasmtime/pull/13381)

* Returning a `ThrownException` when there wasn't a pending exception within a
  store has been fixed.
  [#13306](https://github.com/bytecodealliance/wasmtime/pull/13306)

* A DRC corruption issue when overwriting an i31ref slot has been fixed.
  [#13307](https://github.com/bytecodealliance/wasmtime/pull/13307)

* Taking a store's exception from a debug handler has been fixed.
  [#13310](https://github.com/bytecodealliance/wasmtime/pull/13310)

* Alignment checks of atomics with Winch have been fixed.
  [#13337](https://github.com/bytecodealliance/wasmtime/pull/13337)

* GC barriers around managing the pending exception within a store have been
  fixed.
  [#13330](https://github.com/bytecodealliance/wasmtime/pull/13330)

* Cranelift's handling of short jumps on some architectures is now improved to
  handle very large basic blocks.
  [#13392](https://github.com/bytecodealliance/wasmtime/pull/13392)

* Cross-component stream copies have been fixed.
  [#13418](https://github.com/bytecodealliance/wasmtime/pull/13418)

* Cranelift-generated stack maps have been fixed in a few cases.
  [#13449](https://github.com/bytecodealliance/wasmtime/pull/13449)
  [#13466](https://github.com/bytecodealliance/wasmtime/pull/13466)
  [#13498](https://github.com/bytecodealliance/wasmtime/pull/13498)

* Component-to-component adapters which use resources in arguments and disable
  `concurrency_support` have been fixed.
  [#13542](https://github.com/bytecodealliance/wasmtime/pull/13542)

* A panic in `substituted_component_type` has been fixed when guests have
  exported resources.
  [#13608](https://github.com/bytecodealliance/wasmtime/pull/13608)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
