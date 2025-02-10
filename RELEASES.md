## 30.0.0

Unreleased.

### Added

* New `wasmtime-wasi-io` crate provides a `#![no_std]` wasi:io implementation,
  factored out of `wasmtime-wasi`. Users of `wasmtime-wasi` don't have to
  depend on this new crate.
  [#10036](https://github.com/bytecodealliance/wasmtime/pull/10036)

* Wasmtime's interpreter, Pulley, is now complete and has been listed as
  [tier 2].
  [#9897](https://github.com/bytecodealliance/wasmtime/pull/9897)
  [#9884](https://github.com/bytecodealliance/wasmtime/pull/9884)
  [#9943](https://github.com/bytecodealliance/wasmtime/pull/9943)
  [#9944](https://github.com/bytecodealliance/wasmtime/pull/9944)
  [#9983](https://github.com/bytecodealliance/wasmtime/pull/9983)
  [#9966](https://github.com/bytecodealliance/wasmtime/pull/9966)
  [#9935](https://github.com/bytecodealliance/wasmtime/pull/9935)
  [#10034](https://github.com/bytecodealliance/wasmtime/pull/10034)
  [#10057](https://github.com/bytecodealliance/wasmtime/pull/10057)
  [#10095](https://github.com/bytecodealliance/wasmtime/pull/10095)

* Wasmtime's CI now checks that the repository builds for `aarch64-apple-ios`.
  Note that no tests are run for this target, so it's still [tier 3].
  [#9888](https://github.com/bytecodealliance/wasmtime/pull/9888)

* Winch's support for AArch64 and simd on x64 have continued to progress
  well. Winch additionally now fully supports the `threads` WebAssembly
  proposal.
  [#9889](https://github.com/bytecodealliance/wasmtime/pull/9889)
  [#9970](https://github.com/bytecodealliance/wasmtime/pull/9970)
  [#9950](https://github.com/bytecodealliance/wasmtime/pull/9950)
  [#9987](https://github.com/bytecodealliance/wasmtime/pull/9987)
  [#9990](https://github.com/bytecodealliance/wasmtime/pull/9990)
  [#9959](https://github.com/bytecodealliance/wasmtime/pull/9959)
  [#10008](https://github.com/bytecodealliance/wasmtime/pull/10008)
  [#10028](https://github.com/bytecodealliance/wasmtime/pull/10028)
  [#10029](https://github.com/bytecodealliance/wasmtime/pull/10029)
  [#10023](https://github.com/bytecodealliance/wasmtime/pull/10023)
  [#10042](https://github.com/bytecodealliance/wasmtime/pull/10042)
  [#10050](https://github.com/bytecodealliance/wasmtime/pull/10050)
  [#10039](https://github.com/bytecodealliance/wasmtime/pull/10039)
  [#10082](https://github.com/bytecodealliance/wasmtime/pull/10082)
  [#10092](https://github.com/bytecodealliance/wasmtime/pull/10092)
  [#10109](https://github.com/bytecodealliance/wasmtime/pull/10109)
  [#10148](https://github.com/bytecodealliance/wasmtime/pull/10148)
  [#10147](https://github.com/bytecodealliance/wasmtime/pull/10147)

* The `memory64` WebAssembly feature is now enabled by default. This WebAssembly
  proposal is now considered a [tier 1] feature.
  [#9937](https://github.com/bytecodealliance/wasmtime/pull/9937)
  [#10159](https://github.com/bytecodealliance/wasmtime/pull/10159)

* Wasmtime's full test suite and CI now includes 32-bit platforms such as x86
  and armv7 Linux. These platforms have been added to [tier 3] status and use
  Pulley as their execution backend.
  [#10025](https://github.com/bytecodealliance/wasmtime/pull/10025)

* Initial experimental support for WASIp3 and async features of the Component
  Model have started to land. These features are not yet ready for
  general-purpose use.
  [#10044](https://github.com/bytecodealliance/wasmtime/pull/10044)
  [#10047](https://github.com/bytecodealliance/wasmtime/pull/10047)
  [#10083](https://github.com/bytecodealliance/wasmtime/pull/10083)
  [#10103](https://github.com/bytecodealliance/wasmtime/pull/10103)

* The `wasmtime` CLI now supports using a TOML configuration file via `--config`
  in addition to CLI options.
  [#9811](https://github.com/bytecodealliance/wasmtime/pull/9811)
  [#10132](https://github.com/bytecodealliance/wasmtime/pull/10132)

* Initial support for a new assembler on x64 has been added.
  [#10110](https://github.com/bytecodealliance/wasmtime/pull/10110)
  [#10178](https://github.com/bytecodealliance/wasmtime/pull/10178)

### Changed

* `wasmtime-wasi` split the `WasiView` trait into `IoView` and `WasiView`, and
  `wasmtime-wasi-http` re-uses `IoView` in `WasiHttpView`. Details on porting
  for embedders in PR.
  [#10016](https://github.com/bytecodealliance/wasmtime/pull/10016)

* `wasmtime-wasi` renamed some exported types and traits. Embedders which use
  `Pollable`, `InputStream`, `OutputStream`, `Subscribe`, `HostInputStream`,
  `HostOutputStream`, `PollableFuture`, or `ClosureFuture` from that crate
  will need to rename those imports to their new names, describe in PR.
  [#10036](https://github.com/bytecodealliance/wasmtime/pull/10036)

* Components using a 64-bit linear memory should never have worked before, but
  they're now rejected earlier in the validation process.
  [#9952](https://github.com/bytecodealliance/wasmtime/pull/9952)

* Module validation is now deterministic in the face of multiple errors.
  [#9947](https://github.com/bytecodealliance/wasmtime/pull/9947)

* Wasmtime's minimum supported version of Rust is now 1.82.0.
  [#9956](https://github.com/bytecodealliance/wasmtime/pull/9956)

* Cranelift will now deduplicate `trap[n]z` instructions.
  [#10004](https://github.com/bytecodealliance/wasmtime/pull/10004)

* The `--emit-clif` option to `wasmtime compile` now emits post-optimization
  CLIF.
  [#10011](https://github.com/bytecodealliance/wasmtime/pull/10011)

* The `signals-based-traps` Cargo feature has been removed in favor of
  auto-detection of available features based on the `#[cfg]` directives
  available for the target platform.
  [#9941](https://github.com/bytecodealliance/wasmtime/pull/9941)

* The `async_stack_zeroing` configuration knob now covers all stack allocations,
  not just those from the pooling allocator.
  [#10027](https://github.com/bytecodealliance/wasmtime/pull/10027)

* Wasmtime should work-by-default on more platforms, even those where Cranelift
  has no support for the architecture. This is done by ensuring some
  architecture and platform-specific bits are removed on unknown platforms (and
  Pulley is used instead).
  [#10107](https://github.com/bytecodealliance/wasmtime/pull/10107)

* Wasmtime now compiles on platforms missing 64-bit atomics.
  [#10134](https://github.com/bytecodealliance/wasmtime/pull/10134)

[tier 1]: https://docs.wasmtime.dev/stability-tiers.html#tier-1
[tier 2]: https://docs.wasmtime.dev/stability-tiers.html#tier-2
[tier 3]: https://docs.wasmtime.dev/stability-tiers.html#tier-3

### Fixed

* Fixed a missing case for `Ref::matches_ty` should return `true`.
  [#9985](https://github.com/bytecodealliance/wasmtime/pull/9985)

* A bug with using the `single_pass` register allocation algorithm on x64/s390x
  has been fixed by refactoring how branches are represented.
  [#10086](https://github.com/bytecodealliance/wasmtime/pull/10086)
  [#10087](https://github.com/bytecodealliance/wasmtime/pull/10087)

* A bug with argument extensions on riscv64 has been fixed.
  [#10069](https://github.com/bytecodealliance/wasmtime/pull/10069)

* The `PartialEq` implementation for `RegisteredType` has been fixed.
  [#10091](https://github.com/bytecodealliance/wasmtime/pull/10091)

* The output of `component::bindgen!` now works with `#![no_std]` crates.
  [#10105](https://github.com/bytecodealliance/wasmtime/pull/10105)

* Fix `wasmtime wast` when combined with `--fuel`.
  [#10121](https://github.com/bytecodealliance/wasmtime/pull/10121)

* The `wat` feature of the C API is now plumbed correctly in a few more
  locations.
  [#10124](https://github.com/bytecodealliance/wasmtime/pull/10124)

* Spurious wake-ups in `blocking_*` methods of `InputStream` and `OutputStream`
  have been fixed.
  [#10113](https://github.com/bytecodealliance/wasmtime/pull/10113)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
