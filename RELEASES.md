## 39.0.2

Released 2026-01-14.

### Fixed

* A possible stack overflow in the x64 backend with `cmp` emission has been
  fixed.
  [#12333](https://github.com/bytecodealliance/wasmtime/pull/12333)

--------------------------------------------------------------------------------

## 39.0.1

Released 2025-11-24.

### Fixed

* Compiling the `debug` feature without the `gc` feature enabled has been fixed.
  [#12074](https://github.com/bytecodealliance/wasmtime/pull/12074)

--------------------------------------------------------------------------------

## 39.0.0

Released 2025-11-20.

### Added

* Initial work has begun to support WebAssembly-level debugging natively in
  Wasmtime. This is intended to complement today's preexisting DWARF-level
  debugging, but this work will be portable and operate at the WebAssembly level
  of abstraction rather than the machine-level. Note that this work is not yet
  complete at this time but is expected to get filled out over the coming
  releases.
  [#11768](https://github.com/bytecodealliance/wasmtime/pull/11768)
  [#11769](https://github.com/bytecodealliance/wasmtime/pull/11769)
  [#11873](https://github.com/bytecodealliance/wasmtime/pull/11873)
  [#11892](https://github.com/bytecodealliance/wasmtime/pull/11892)
  [#11895](https://github.com/bytecodealliance/wasmtime/pull/11895)

* The pooling allocator now exposes more metrics about unused slots.
  [#11789](https://github.com/bytecodealliance/wasmtime/pull/11789)

* The [Wizer] and [component-init] projects have been merged into Wasmtime under
  a new `wasmtime wizer` CLI subcommand and `wasmtime-wizer` crate. This is
  mostly a drop-in replacement for both with a minor caveat that the
  initialization function is now called `wizer-initialize` instead of
  `wizer.initialize` to be compatible with components.
  [#11805](https://github.com/bytecodealliance/wasmtime/pull/11805)
  [#11851](https://github.com/bytecodealliance/wasmtime/pull/11851)
  [#11853](https://github.com/bytecodealliance/wasmtime/pull/11853)
  [#11855](https://github.com/bytecodealliance/wasmtime/pull/11855)
  [#11857](https://github.com/bytecodealliance/wasmtime/pull/11857)
  [#11863](https://github.com/bytecodealliance/wasmtime/pull/11863)
  [#11866](https://github.com/bytecodealliance/wasmtime/pull/11866)
  [#11867](https://github.com/bytecodealliance/wasmtime/pull/11867)
  [#11877](https://github.com/bytecodealliance/wasmtime/pull/11877)
  [#11876](https://github.com/bytecodealliance/wasmtime/pull/11876)
  [#11878](https://github.com/bytecodealliance/wasmtime/pull/11878)
  [#11891](https://github.com/bytecodealliance/wasmtime/pull/11891)
  [#11897](https://github.com/bytecodealliance/wasmtime/pull/11897)
  [#11898](https://github.com/bytecodealliance/wasmtime/pull/11898)

[Wizer]: https://github.com/bytecodealliance/wizer
[component-init]: https://github.com/dicej/component-init

* The `Config::wasm_feature` method is now public.
  [#11812](https://github.com/bytecodealliance/wasmtime/pull/11812)

* Enabling the wasm exceptions proposal is now exposed in the C API.
  [#11861](https://github.com/bytecodealliance/wasmtime/pull/11861)

* The `wasmtime` crate now has a `custom-sync-primitives` Cargo feature which
  enables using custom synchronization primitives defined by the embedder. This
  is useful in `no_std` targets where the default panic-on-contention primitives
  are not appropriate.
  [#11836](https://github.com/bytecodealliance/wasmtime/pull/11836)

* Wasmtime now supports unsafe intrinsics to be used for compile-time builtins.
  This can be used to provide give low-level access to host APIs/memory to a
  guest program in a controlled fashion.
  [#11825](https://github.com/bytecodealliance/wasmtime/pull/11825)
  [#11918](https://github.com/bytecodealliance/wasmtime/pull/11918)

* The `signals_based_traps` configuration option is now exposed in the C API.
  [#11879](https://github.com/bytecodealliance/wasmtime/pull/11879)

* A new `EqRef::from_i31` function has been added.
  [#11884](https://github.com/bytecodealliance/wasmtime/pull/11884)

* The `wasmtime serve` subcommand will, by default, now reuse instances when
  used with WASIp3 components. This increases throughput and additionally
  showcases the concurrent features of WASIp3. This can be opted-out-of on the
  CLI as well.
  [#11807](https://github.com/bytecodealliance/wasmtime/pull/11807)

* The C++ API for components has been filled out and implemented.
  [#11880](https://github.com/bytecodealliance/wasmtime/pull/11880)
  [#11889](https://github.com/bytecodealliance/wasmtime/pull/11889)
  [#11988](https://github.com/bytecodealliance/wasmtime/pull/11988)

* A new `ResourceDynamic` type, similar to `Resource<T>`, has been added to
  support host resources that have a dynamic tag at runtime rather than a
  statically known tag at compile time. This is then used to implement resources
  in the C/C++ API as well.
  [#11885](https://github.com/bytecodealliance/wasmtime/pull/11885)
  [#11920](https://github.com/bytecodealliance/wasmtime/pull/11920)

* The C/C++ API of Wasmtime now supports the custom-page-sizes wasm proposal.
  [#11890](https://github.com/bytecodealliance/wasmtime/pull/11890)

* Initial support has been added for the cooperative multithreading component
  model proposal in Wasmtime, built on async primitives.
  [#11751](https://github.com/bytecodealliance/wasmtime/pull/11751)

* The `epoch_deadline_callback` Rust API has been bound in C++.
  [#11945](https://github.com/bytecodealliance/wasmtime/pull/11945)

* A new `Request::into_http` helper has been added to the WASIp3 implementation
  of `wasi:http`.
  [#11843](https://github.com/bytecodealliance/wasmtime/pull/11843)

* A `define_unknown_imports_as_traps` function has been added to the C API.
  [#11962](https://github.com/bytecodealliance/wasmtime/pull/11962)

* A callback-based implementation of `stdout` and `stderr` has been added to the
  C API for WASI configuration.
  [#11965](https://github.com/bytecodealliance/wasmtime/pull/11965)

### Changed

* Running async functions in the component model now operates at the
  `Store`-level of abstraction rather than an `Instance`.
  [#11796](https://github.com/bytecodealliance/wasmtime/pull/11796)

* The `wasmtime serve` subcommand no longer mistakenly spawns an epoch thread
  per-request and instead uses a single epoch thread.
  [#11817](https://github.com/bytecodealliance/wasmtime/pull/11817)

* The `component-model-async` Cargo feature is now on-by-default. Note that it
  is still gated at runtime by default. Also note that Wasmtime 39 does not include
  [#12031](https://github.com/bytecodealliance/wasmtime/pull/12031) which means
  that components using async produced with the latest `wasm-tools` will not run
  in Wasmtime 39. To run async components it's recommended to pin to a
  historical version of `wasm-tools` and guest toolchains for now.
  [#11822](https://github.com/bytecodealliance/wasmtime/pull/11822)

* Bindings generated by `wiggle` no longer use `async_trait`.
  [#11839](https://github.com/bytecodealliance/wasmtime/pull/11839)

* Wasmtime's documentation now has an example of a plugin system using Wasmtime.
  [#11848](https://github.com/bytecodealliance/wasmtime/pull/11848)

* Profiling with perfmap or jitdump now uses `O_APPEND` to be more amenable to
  other engines in the same process also using perfmap/jitdump.
  [#11865](https://github.com/bytecodealliance/wasmtime/pull/11865)

* The `wasmtime-wasi-http` crate now uses `UnsyncBoxBody` to clarify that `Sync`
  is not required.
  [#11941](https://github.com/bytecodealliance/wasmtime/pull/11941)

* A `.` character is now used instead of `/` int he `bindgen!` macro to separate
  interface members.
  [#11947](https://github.com/bytecodealliance/wasmtime/pull/11947)

* The `func_new` function for component linkers now provides the type to the
  callee so it knows the type that the component that imported it is using.
  [#11944](https://github.com/bytecodealliance/wasmtime/pull/11944)

* The `component::Func` type now has a type accessor and the old params/result
  accessors were deleted.
  [#11943](https://github.com/bytecodealliance/wasmtime/pull/11943)

* Wasmtime now requires Rust 1.89.0 or later to compile.
  [#11959](https://github.com/bytecodealliance/wasmtime/pull/11959)

### Fixed

* Some panics handling shapes of components with resources in various locations
  has been fixed.
  [#11798](https://github.com/bytecodealliance/wasmtime/pull/11798)

* Bitwise float operations in Cranelift have been fixed on aarch64.
  [#11811](https://github.com/bytecodealliance/wasmtime/pull/11811)

* An off-by-one in the bounds check of wasm atomic operations has been fixed.
  [#11977](https://github.com/bytecodealliance/wasmtime/pull/11977)

* Bounds-check elision now happens again with 4 GiB guard pages.
  [#11973](https://github.com/bytecodealliance/wasmtime/pull/11973)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
