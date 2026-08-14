## 48.0.1

Unreleased.

### Fixed

* Context slots in component compositions are now correctly managed.
  [#14139](https://github.com/bytecodealliance/wasmtime/pull/14139)

--------------------------------------------------------------------------------

## 48.0.0

Released 2026-08-20.

### Added

* Wasmtime has an initial implementation of the component model
  fixed-length-lists feature.
  [#12315](https://github.com/bytecodealliance/wasmtime/pull/12315)

* Wasmtime's reflection of component imports/exports now exposes `external-id`
  information.
  [#13874](https://github.com/bytecodealliance/wasmtime/pull/13874)

* Cranelift's alias analysis pass now eliminates dead stores.
  [#13806](https://github.com/bytecodealliance/wasmtime/pull/13806)
  [#13947](https://github.com/bytecodealliance/wasmtime/pull/13947)

* Winch now supports the wasm SIMD proposal on AArch64.
  [#13911](https://github.com/bytecodealliance/wasmtime/pull/13911)
  [#13921](https://github.com/bytecodealliance/wasmtime/pull/13921)
  [#13928](https://github.com/bytecodealliance/wasmtime/pull/13928)
  [#13937](https://github.com/bytecodealliance/wasmtime/pull/13937)
  [#13945](https://github.com/bytecodealliance/wasmtime/pull/13945)
  [#13938](https://github.com/bytecodealliance/wasmtime/pull/13938)
  [#13946](https://github.com/bytecodealliance/wasmtime/pull/13946)
  ...

* Cranelift now supports an AVX512-VNNI lowering for the dot-product wasm
  instruction and `usdot` on AArch64.
  [#14006](https://github.com/bytecodealliance/wasmtime/pull/14006)
  [#14054](https://github.com/bytecodealliance/wasmtime/pull/14054)

* Wasmtime's `bindgen!` macro now has an `include_component_type` option to
  generate a `COMPONENT_TYPE` constant with the encoded type of the world being
  bound.
  [#14013](https://github.com/bytecodealliance/wasmtime/pull/14013)

* Wasmtime now supports configurable fuel costs for variable-length wasm
  opcodes.
  [#13931](https://github.com/bytecodealliance/wasmtime/pull/13931)

### Changed

* Host-implemented traits in `wasmtime-wasi-http` are now the same across
  wasip2/wasip3 and no longer require separate implementations/structures.
  [#13810](https://github.com/bytecodealliance/wasmtime/pull/13810)
  [#13812](https://github.com/bytecodealliance/wasmtime/pull/13812)
  [#13835](https://github.com/bytecodealliance/wasmtime/pull/13835)

* Wasmtime will now use the `process_madvise` syscall on Linux where available
  which can improve the performance of the pooling allocator when the
  deallocation batch size is configured to larger than 1.
  [#13830](https://github.com/bytecodealliance/wasmtime/pull/13830)

* Synchronous cancellation of streams/futures/subtasks now traps if the waitable
  was already in a waitable set.
  [#13708](https://github.com/bytecodealliance/wasmtime/pull/13708)

* Winch is now flagged to be compatible with component-model-async.
  [#13845](https://github.com/bytecodealliance/wasmtime/pull/13845)

* Wasmtime's pooling allocator now uses more sharding to reduce lock contention.
  [#13840](https://github.com/bytecodealliance/wasmtime/pull/13840)

* Wasmtime now requires Rust 1.95.0 to build.
  [#13853](https://github.com/bytecodealliance/wasmtime/pull/13853)

* Cranelift now has a uniform maximum size across all backends on the bounds of
  a function's stack frame.
  [#13783](https://github.com/bytecodealliance/wasmtime/pull/13783)

* Wasmtime's `LinkerInstance` for components now supports being reopened to
  gradually add more items.
  [#13908](https://github.com/bytecodealliance/wasmtime/pull/13908)

* The `wasmtime-wasi` crate's default configuration now denies creation of
  TCP/UDP sockets by default.
  [#13936](https://github.com/bytecodealliance/wasmtime/pull/13936)

* Wasmtime now configures async task context fields on `realloc` calls to zero.
  [#13949](https://github.com/bytecodealliance/wasmtime/pull/13949)

* Work has continued on continuous verification of Cranelift's lowering rules
  on AArch64.
  [#13935](https://github.com/bytecodealliance/wasmtime/pull/13935)
  [#13929](https://github.com/bytecodealliance/wasmtime/pull/13929)
  [#13998](https://github.com/bytecodealliance/wasmtime/pull/13998)

* Codegen for loads on AArch64 has been optimized slightly to improve sharing
  common sub-expressions.
  [#13766](https://github.com/bytecodealliance/wasmtime/pull/13766)

* Work continues on implementing the stack-switching proposal.
  [#11717](https://github.com/bytecodealliance/wasmtime/pull/11717)
  [#13996](https://github.com/bytecodealliance/wasmtime/pull/13996)
  [#14052](https://github.com/bytecodealliance/wasmtime/pull/14052)

* Permissions for `wasi-filesystem` in the implementation of the `wasmtime-wasi`
  crate have been simplified to either read-write or read-only for a directory.
  [#14010](https://github.com/bytecodealliance/wasmtime/pull/14010)

### Fixed

* A late-drop of host-defined stream producers/consumers has been fixed.
  [#13891](https://github.com/bytecodealliance/wasmtime/pull/13891)

* Call hooks are now invoked around async yields when dealing with concurrent
  execution.
  [#13871](https://github.com/bytecodealliance/wasmtime/pull/13871)

* Extreme filesystem timestamps no longer cause panics.
  [#13894](https://github.com/bytecodealliance/wasmtime/pull/13894)

* An erroneous trap was fixed where an async-delivered write-closed event was
  sent to a future.
  [#13914](https://github.com/bytecodealliance/wasmtime/pull/13914)

* An erroneous trap where Wasmtime internally used the wrong thread id has been
  fixed.
  [#13926](https://github.com/bytecodealliance/wasmtime/pull/13926)

* Cross-component instance streams are no longer erroneously flagged as being
  intra-instance.
  [#14018](https://github.com/bytecodealliance/wasmtime/pull/14018)

* A panic in the `wasmtime` CLI with non-utf8 environment variables has been
  fixed.
  [#14017](https://github.com/bytecodealliance/wasmtime/pull/14017)

* Atomic waits on big-endian hosts have been fixed.
  [#14027](https://github.com/bytecodealliance/wasmtime/pull/14027)

* Enabling MPK with CoW images has been fixed.
  [#14076](https://github.com/bytecodealliance/wasmtime/pull/14076)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
* [47.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-47.0.0/RELEASES.md)
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
