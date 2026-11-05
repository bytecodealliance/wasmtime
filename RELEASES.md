## 51.0.0

Unreleased.

### Added

* Cranelift has initial support for 128-bit atomics on AArch64.
  [#14475](https://github.com/bytecodealliance/wasmtime/pull/14475)

* Cranelift has initial support for 128-bit atomics on s390x.
  [#14771](https://github.com/bytecodealliance/wasmtime/pull/14771)
  [#14809](https://github.com/bytecodealliance/wasmtime/pull/14809)

* Cranelift now has an `interrupt_poll` instruction suitable for generating a
  native signal with an architectural load which the embedder has necessary
  metadata to redirect execution if necessary.
  [#14579](https://github.com/bytecodealliance/wasmtime/pull/14579)

* Wasmtime now has the ability to precompute symbol tables with the
  `GuestProfiler` to greatly improve the cost of capturing profiles across many
  stores of the same module's intances.
  [#14753](https://github.com/bytecodealliance/wasmtime/pull/14753)

* Wasmtime now supports filling in unknown imports with functions that return
  default values.
  [#14746](https://github.com/bytecodealliance/wasmtime/pull/14746)

* The `WorkerState` trait in `wasmtime-wais-http` now has a `poll` method which
  can be overridden to hook into polling of guest-based futures.
  [#14847](https://github.com/bytecodealliance/wasmtime/pull/14847)

* The WIT `map` type can now be used from the C API.
  [#14840](https://github.com/bytecodealliance/wasmtime/pull/14840)

* The host Rust type used for a WIT `map` type's bindings can now be customized
  in `bindgen!`.
  [#14927](https://github.com/bytecodealliance/wasmtime/pull/14927)

* A new `Global::get_numeric` function allows getting the value of a `Global`
  with a `StoreContext`, but only for numeric values.
  [#14979](https://github.com/bytecodealliance/wasmtime/pull/14979)

* The `wasmtime serve` subcommand now has a `--post-return-timeout` option.
  [#14880](https://github.com/bytecodealliance/wasmtime/pull/14880)

* Cranelift now has a `volatile` memory flag.
  [#14999](https://github.com/bytecodealliance/wasmtime/pull/14999)

* Wasmtime and the CLI now support configuring buffer sizes in WASI to be used
  instead of having these values hardcoded.
  [#15095](https://github.com/bytecodealliance/wasmtime/pull/15095)

### Changed

* Compile-time for may host-defined functions should be improved by a good
  amount.
  [#14517](https://github.com/bytecodealliance/wasmtime/pull/14517)

* The performance of returning values from a host function that will never call
  `realloc` in the guest has been significantly improved.
  [#14468](https://github.com/bytecodealliance/wasmtime/pull/14468)

* The `{Memory,Table}Type::new` APIs now return a `Result` instead of panicking
  to represent fallible creation.
  [#14591](https://github.com/bytecodealliance/wasmtime/pull/14591)

* Buffering of outgoing wasip3 http requests is now configurable.
  [#14613](https://github.com/bytecodealliance/wasmtime/pull/14613)

* Cranelift now rejects too-large stack slot alignment.
  [#14650](https://github.com/bytecodealliance/wasmtime/pull/14650)

* Cranelift now rejects struct-args in the `tail` calling convention as this
  was never actually supported.
  [#14649](https://github.com/bytecodealliance/wasmtime/pull/14649)

* The behavior of removing directories with a trailing slash has been fixed.
  [#14662](https://github.com/bytecodealliance/wasmtime/pull/14662)
  [#14663](https://github.com/bytecodealliance/wasmtime/pull/14663)

* The shared global code register is now omitted in no-singals-based-traps
  builds of Wasmtime.
  [#14690](https://github.com/bytecodealliance/wasmtime/pull/14690)

* The `Direct{Source,Destination}::remaining` APIs now return a `Result`.
  [#14740](https://github.com/bytecodealliance/wasmtime/pull/14740)

* Cranelift's lowering for `{usub,sadd,ssub}_overflow` combined with a branch
  has been optimized.
  [#14717](https://github.com/bytecodealliance/wasmtime/pull/14717)

* Cranelift's lowering of the `shuffle` instruction is now more optimal when it
  can use `pshufb` with just a single operand.
  [#14849](https://github.com/bytecodealliance/wasmtime/pull/14849)

* Wasmtime's `LinearMemory` trait has been adjusted to be usable with more
  embedding options.
  [#14885](https://github.com/bytecodealliance/wasmtime/pull/14885)

* The `wasmtime serve` will now use graceful shutdown when receiving a SIGTERM
  signal on Unix.
  [#14904](https://github.com/bytecodealliance/wasmtime/pull/14904)

* Enabling the WebAssembly simd or relaxed-simd features on riscv64 now requires
  the `has_v` extension to be enabled, and otherwise an error is returned.
  [#14897](https://github.com/bytecodealliance/wasmtime/pull/14897)

* The number of active fibers within a store is now limited by default.
  [#14971](https://github.com/bytecodealliance/wasmtime/pull/14971)

* Usage of signal handlers is now automatically disabled on targets where
  Wasmtime does not have an implementation of pulling out context information.
  [#14990](https://github.com/bytecodealliance/wasmtime/pull/14990)

* The `ref.func` instruction's implementation is now more optimal.
  [#14683](https://github.com/bytecodealliance/wasmtime/pull/14683)

* Wasmtime will no longer provide precompiled "min" artifacts for all platforms
  in CI, just a subset.
  [#15127](https://github.com/bytecodealliance/wasmtime/pull/15127)

### Fixed

* Zero-length writes in WASIp2 no longer trap.
  [#14533](https://github.com/bytecodealliance/wasmtime/pull/14533)

* Cache maintenance on AArch64 platforms has been improved.
  [#14454](https://github.com/bytecodealliance/wasmtime/pull/14454)

* Component-to-component calls now more strictly handle reentrance and
  `may_leave` conditions according to the spec.
  [#14542](https://github.com/bytecodealliance/wasmtime/pull/14542)
  [#14574](https://github.com/bytecodealliance/wasmtime/pull/14574)
  [#14670](https://github.com/bytecodealliance/wasmtime/pull/14670)

* Components no longer interpret an embedder-returned `ThrownException` error as
  a raised exception within the component itself.
  [#14549](https://github.com/bytecodealliance/wasmtime/pull/14549)
  [#14688](https://github.com/bytecodealliance/wasmtime/pull/14688)
  [#14689](https://github.com/bytecodealliance/wasmtime/pull/14689)

* More checks are in place to insulate embedders against incorrect management of
  futures/streams to avoid possibly invalid states a guest may see.
  [#14552](https://github.com/bytecodealliance/wasmtime/pull/14552)

* Component model guests now correctly trap if `borrow<T>` values are still live
  when an `async` function returns.
  [#14580](https://github.com/bytecodealliance/wasmtime/pull/14580)

* Fixed a resource leak with the WASIp2 `outgoing-handler.handle` function.
  [#14603](https://github.com/bytecodealliance/wasmtime/pull/14603)

* The youngest frame in a stack overflow is now handled better on s390x and
  Pulley.
  [#14687](https://github.com/bytecodealliance/wasmtime/pull/14687)

* This release contains a large number of tweaks, adjustments, and primarily
  fixes for component-model-async behavior. Most of this falls in the category
  of ensuring that Wasmtime's behavior matches the spec's behavior, but this
  also includes fixes to host-specific interactions, such as the consequence of
  cancelling/dropping futures, to make Wasmtime more robust in these situations.

* Similarly this release contains a large number of similar tweaks, adjustments,
  and fixes for HTTP-related behavior in `wasmtime-wasi-http` and `wasmtime
  serve`.

* Debugging guests with gdbstub now correctly forwards interrupt requests to the
  debuggee.
  [#14776](https://github.com/bytecodealliance/wasmtime/pull/14776)

* Handling of component type ascription on exports has been fixed.
  [#14865](https://github.com/bytecodealliance/wasmtime/pull/14865)

* Wasmtime's usage of the `PAGEMAP_SCAN` ioctl has been fixed on newer versions
  of Linux kernels.
  [#15134](https://github.com/bytecodealliance/wasmtime/pull/15134)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
* [50.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-50.0.0/RELEASES.md)
* [49.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-49.0.0/RELEASES.md)
* [48.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-48.0.0/RELEASES.md)
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
