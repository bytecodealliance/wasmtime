--------------------------------------------------------------------------------

## 22.0.0

Unreleased.

### Added

### Changed

--------------------------------------------------------------------------------

## 21.0.0

Unreleased.

### Added

* The `wasmtime explore` subcommand now supports `*.wat` files.
  [#8314](https://github.com/bytecodealliance/wasmtime/issues/8314)

* Wasmtime now supports DWARF Fission `*.dwp` files.
  [#8055](https://github.com/bytecodealliance/wasmtime/issues/8055)

* The `wasmtime` crate now supports `#![no_std]`, and more information about
  this transition can be found in
  [#8341](https://github.com/bytecodealliance/wasmtime/issues/8341).
  [#8463](https://github.com/bytecodealliance/wasmtime/issues/8463)
  [#8483](https://github.com/bytecodealliance/wasmtime/issues/8483)
  [#8485](https://github.com/bytecodealliance/wasmtime/issues/8485)
  [#8528](https://github.com/bytecodealliance/wasmtime/issues/8528)
  [#8533](https://github.com/bytecodealliance/wasmtime/issues/8533)

* A `Config` knob is now available for one-entry `call_indirect` caching to
  speed up modules with lots of `call_indirect` instructions.
  [#8509](https://github.com/bytecodealliance/wasmtime/issues/8509)

* Cranelift's riscv64 backend has initial support for the Zfa Extension.
  [#8536](https://github.com/bytecodealliance/wasmtime/issues/8536)

* The WebAssembly `tail-calls` proposal is now enabled by default when using the
  Cranelift backend, except for the s390x architecture.
  [#8540](https://github.com/bytecodealliance/wasmtime/issues/8540)

### Changed

* Support for NaN canonicalization on x64 has been optimized to avoid branches.
  [#8313](https://github.com/bytecodealliance/wasmtime/issues/8313)

* The `bindgen!` macro now defaults to assuming imports cannot trap, but this
  behavior is configurable at macro-generation time.
  [#8310](https://github.com/bytecodealliance/wasmtime/issues/8310)

* The `fd_{read,write}` implementations in wasmtime-wasi have been optimized.
  [#8303](https://github.com/bytecodealliance/wasmtime/issues/8303)

* The `wasmtime-wasi-http` crate has been refactored and has improved
  documentation.
  [#8332](https://github.com/bytecodealliance/wasmtime/issues/8332)
  [#8347](https://github.com/bytecodealliance/wasmtime/issues/8347)

* Unused `with` parameters in `bindgen!` now generate an error.
  [#8371](https://github.com/bytecodealliance/wasmtime/issues/8371)

* The `fd_read` implementation in wasmtime-wasi now only reads into a single
  iovec per call.
  [#8415](https://github.com/bytecodealliance/wasmtime/issues/8415)

* The `wasmtime_val_t` type in the C API no longer holds any allocations. This
  type must still be manually managed to properly unroot values, however.
  [#8451](https://github.com/bytecodealliance/wasmtime/issues/8451)

* Add an optimized "sleep" path for `poll_oneoff` to wasmtime-wasi.
  [#8429](https://github.com/bytecodealliance/wasmtime/issues/8429)

* The Wasmtime-internal `wasmtime-runtime` crate has been removed.
  [#8501](https://github.com/bytecodealliance/wasmtime/issues/8501)

* The pooling allocator now allows twice as many elements as before.
  [#8527](https://github.com/bytecodealliance/wasmtime/issues/8527)

* Using CMake to build the C API has been improved along a number of axes.
  [#8490](https://github.com/bytecodealliance/wasmtime/issues/8490)
  [#8497](https://github.com/bytecodealliance/wasmtime/issues/8497)
  [#8549](https://github.com/bytecodealliance/wasmtime/issues/8549)

### Fixed

* Pooling allocator CLI options are now respected with the `wasmtime serve`
  subcommand.
  [#8525](https://github.com/bytecodealliance/wasmtime/issues/8525)

--------------------------------------------------------------------------------

## 20.0.2

Released 2024-05-07.

### Added

* Improve error in CMake for when Cargo is not found.
  [#8497](https://github.com/bytecodealliance/wasmtime/issues/8497)

* Use `--release` in CMake with MinSizeRel and RelWithDebInfo.
  [#8549](https://github.com/bytecodealliance/wasmtime/issues/8549)

* Add a `WASMTIME_FASTEST_RUNTIME` configuration option for CMake which enables
  LTO and other related optimization options.
  [#8554](https://github.com/bytecodealliance/wasmtime/issues/8554)

--------------------------------------------------------------------------------

## 20.0.1

Released 2024-05-03.

### Added

* Make the c-api CMakeLists.txt easier to import.
  [#8496](https://github.com/bytecodealliance/wasmtime/issues/8496)

--------------------------------------------------------------------------------

## 20.0.0

Released 2024-04-22

### Added

* Support for shared WebAssembly memories has been added to the C API.
  [#7940](https://github.com/bytecodealliance/wasmtime/issues/7940)

* A `Component::component_type` method has been added to reflect on the imports
  and exports of a component.
  [#8078](https://github.com/bytecodealliance/wasmtime/issues/8078)

* The `with` key in `bindgen!` now supports remapping entire packages and
  namespaces.
  [#8083](https://github.com/bytecodealliance/wasmtime/issues/8083)

* Winch now supports the component model by using Cranelift to generate
  trampolines.
  [#8082](https://github.com/bytecodealliance/wasmtime/issues/8082)
  [#8109](https://github.com/bytecodealliance/wasmtime/issues/8109)

* The WASI-NN backend now supports ONNX.
  [#7691](https://github.com/bytecodealliance/wasmtime/issues/7691)

* The `wasmtime` CLI now has an `-S inherit-env` flag for inheriting the entire
  process environment.
  [#8168](https://github.com/bytecodealliance/wasmtime/issues/8168)

* Winch now supports the WebAssembly memory64 proposal.
  [#8194](https://github.com/bytecodealliance/wasmtime/issues/8194)

* Embedders can now opt-in to allowing wasmtime-wasi to block the current thread
  with file operations, for example.
  [#8190](https://github.com/bytecodealliance/wasmtime/issues/8190)

* A `wasmtime::CodeBuilder` type has been added to refine the configuration of
  compiling a `Module` or a `Component`.
  [#8181](https://github.com/bytecodealliance/wasmtime/issues/8181)

* The `wasmtime serve` subcommand now enables configuring preopened directories
  and environment variables.
  [#8279](https://github.com/bytecodealliance/wasmtime/issues/8279)

### Changed

* Support for WebAssembly GC is in the process of being implemented which has
  required refactoring and reimplementing the existing gc support for
  `externref`. Many APIs in this area has changed, see linked PRs for more
  details. Note that the `wasm_*` parts of the C API no longer support
  `externref`.
  [#8011](https://github.com/bytecodealliance/wasmtime/issues/8011)
  [#8196](https://github.com/bytecodealliance/wasmtime/issues/8196)

* The `wasmtime::component::Val` type no longer stores type information and
  instead must be interpreted in the context of a type.
  [#8062](https://github.com/bytecodealliance/wasmtime/issues/8062)

* The arguments required for `ResourceAny::try_from_resource` have been
  simplified by refactoring the internal representation.
  [#8061](https://github.com/bytecodealliance/wasmtime/issues/8061)

* The arguments required for `wasmtime::component::Linker::func_new` have been
  simplified by refactoring the internal representation.
  [#8070](https://github.com/bytecodealliance/wasmtime/issues/8070)

* The pooling allocator is now enabled by default with `wasmtime serve`.
  [#8073](https://github.com/bytecodealliance/wasmtime/issues/8073)

* The error message for missing imports in has been improved with components.
  [#7645](https://github.com/bytecodealliance/wasmtime/issues/7645)

* Wasmtime's MSRV is now 1.75.0.
  [#8205](https://github.com/bytecodealliance/wasmtime/issues/8205)

* Wasmtime's translation of table-related instructions has improved codegen in
  some common cases, especially with `call_indirect`.
  [#8063](https://github.com/bytecodealliance/wasmtime/issues/8063)
  [#8125](https://github.com/bytecodealliance/wasmtime/issues/8125)
  [#8124](https://github.com/bytecodealliance/wasmtime/issues/8124)
  [#8134](https://github.com/bytecodealliance/wasmtime/issues/8134)
  [#8137](https://github.com/bytecodealliance/wasmtime/issues/8137)
  [#8162](https://github.com/bytecodealliance/wasmtime/issues/8162)
  [#8159](https://github.com/bytecodealliance/wasmtime/issues/8159)
  [#8172](https://github.com/bytecodealliance/wasmtime/issues/8172)
  [#8171](https://github.com/bytecodealliance/wasmtime/issues/8171)
  [#8139](https://github.com/bytecodealliance/wasmtime/issues/8139)
  [#8206](https://github.com/bytecodealliance/wasmtime/issues/8206)

* Book-based documentation has been reordered and refactored.
  [#8130](https://github.com/bytecodealliance/wasmtime/issues/8130)

* The `-S common` flag is renamed to `-S cli`, to better reflect that it provides
  the wasi-cli APIs. `-S common` is still accepted for now, and will be deprecated
  in the future.
  [#8166](https://github.com/bytecodealliance/wasmtime/issues/8166)

* The tail-call calling convention now supports callee-saved registers to
  improve performance and allow enabling this WebAssembly proposal by default in
  the future.
  [#8246](https://github.com/bytecodealliance/wasmtime/issues/8246)

* The `wasmtime-wasi` crate has been refactored to restructure some items and
  documentation has been added for most items.
  [#8228](https://github.com/bytecodealliance/wasmtime/issues/8228)

* Support for the WebAssembly `threads` proposal is now gated by an
  on-by-default Cargo feature named `threads`.
  [#8260](https://github.com/bytecodealliance/wasmtime/issues/8260)

* Borrow-checking in `wiggle` has been optimized to not be as fine-grained any
  more. This is a breaking change if users are relying on the ability to safely
  mutably borrow disjoint regions of memory.
  [#8277](https://github.com/bytecodealliance/wasmtime/issues/8277)

### Fixed

* Connection timeouts with `wasi-http` have been fixed.
  [#8085](https://github.com/bytecodealliance/wasmtime/issues/8085)

* Generating bindings with `bindgen!` now works correctly when some WIT types
  are not used.
  [#8065](https://github.com/bytecodealliance/wasmtime/issues/8065)

* Errors in `wasi-http` are no longer accidentally returned as traps.
  [#8272](https://github.com/bytecodealliance/wasmtime/issues/8272)

--------------------------------------------------------------------------------

## 19.0.2

Released 2024-04-11.

* Fix a panic when compiling invalid components.
  [#8323](https://github.com/bytecodealliance/wasmtime/issues/8323)

* Fix `bindgen!` with `trappable_error_type` using unversioned/versioned
  packages.
  [#8305](https://github.com/bytecodealliance/wasmtime/pull/8305)

* cranelift: Include clobbers and outgoing args in stack limit.
  [#8301](https://github.com/bytecodealliance/wasmtime/pull/8301)

--------------------------------------------------------------------------------

## 19.0.1

Released 2024-04-02.

* Fix a panic using tables with the wrong type.
  [#8284](https://github.com/bytecodealliance/wasmtime/pull/8284)

--------------------------------------------------------------------------------

## 19.0.0

Released 2024-03-20

### Added

* C API bindings for the sampling-based profiler in Wasmtime have been added.
  [#7854](https://github.com/bytecodealliance/wasmtime/pull/7854)

* Add the WasiP1Ctx to ease the use of `wasmtime-wasi` in place of `wasi-common`
  [#8053](https://github.com/bytecodealliance/wasmtime/pull/8053)

* The Winch compiler backend is now feature-complete for x64. Note that minor
  issues and fuzz bugs are still being addressed, but now's a good time to test
  if you're interested.
  [#7894](https://github.com/bytecodealliance/wasmtime/pull/7894)
  [#7909](https://github.com/bytecodealliance/wasmtime/pull/7909)
  [#7927](https://github.com/bytecodealliance/wasmtime/pull/7927)
  [#7932](https://github.com/bytecodealliance/wasmtime/pull/7932)
  [#7949](https://github.com/bytecodealliance/wasmtime/pull/7949)
  [#7974](https://github.com/bytecodealliance/wasmtime/pull/7974)
  [#8001](https://github.com/bytecodealliance/wasmtime/pull/8001)

* The typed function references proposal to WebAssembly is now fully
  implemented.
  [#7943](https://github.com/bytecodealliance/wasmtime/pull/7943)

* The `component::Linker` type is now "semver aware" in that it enables loading
  components referring to past or future versions of interfaces so long as the
  types are compatible.
  [#7994](https://github.com/bytecodealliance/wasmtime/pull/7994)

* Wasmtime can now be built for "custom platforms" which is intended for
  bare-metal builds.
  [#7995](https://github.com/bytecodealliance/wasmtime/pull/7995)

* The `wasmtime-wasi-nn` crate now has a backend based on WinML.
  [#7807](https://github.com/bytecodealliance/wasmtime/pull/7807)

* The `wasmtime` CLI now has flags for configuring limits of the pooling
  allocator.
  [#8027](https://github.com/bytecodealliance/wasmtime/pull/8027)


### Changed

* The `wasmtime serve` command no longer binds its port with `SO_REUSEADDR`
  meaning that if it is invoked twice only one will succeed.
  [#7863](https://github.com/bytecodealliance/wasmtime/pull/7863)

* The sampling-based profiler in Wasmtime now takes an explicit argument of
  the time delta between samples.
  [#7873](https://github.com/bytecodealliance/wasmtime/pull/7873)

* Many accessors for type information now require an `&Engine` argument to be
  specified in preparation for an implementation of wasm GC.
  [#7892](https://github.com/bytecodealliance/wasmtime/pull/7892)

* The `host` header is now forbidden in wasi-http.
  [#7905](https://github.com/bytecodealliance/wasmtime/pull/7905)

* Stronger type-checks are now performed for host-owned resources when
  interacting with the component model to help catch mistakes earlier.
  [#7902](https://github.com/bytecodealliance/wasmtime/pull/7902)

* Demangling Rust and C/C++ symbols in WebAssembly modules is now done by
  default in the C API.
  [#7962](https://github.com/bytecodealliance/wasmtime/pull/7962)

* Preview2-based APIs are now located at the root of the `wasmtime_wasi` crate
  instead of a submodule.
  [#7933](https://github.com/bytecodealliance/wasmtime/pull/7933)

* Components now reserve index 0 for handle tables to match the component model
  specification.
  [#7661](https://github.com/bytecodealliance/wasmtime/pull/7661)

* Support for `externref` and similar features has been moved behind a new `gc`
  Cargo feature. This will also gate support for Wasm GC in the future.
  [#7975](https://github.com/bytecodealliance/wasmtime/pull/7975)

* Require the `WASMTIME_WASI_CONFIG_PREOPEN_SOCKET_ALLOW` environment variable
  to bet set to allow the use of `wasi_config_preopen_socket` via the c api, as
  it will be deprecated in `20.0.0`.
  [#8053](https://github.com/bytecodealliance/wasmtime/pull/8053)

### Fixed

* WIT interface names that are Rust keywords now correctly generate bindings.
  [#7790](https://github.com/bytecodealliance/wasmtime/pull/7790)

* PKRU state is now restored across await points.
  [#7789](https://github.com/bytecodealliance/wasmtime/pull/7789)

* Wasmtime now correctly supports `global.get` in all constant expressions
  within a module.
  [#7996](https://github.com/bytecodealliance/wasmtime/pull/7996)

--------------------------------------------------------------------------------

## 18.0.4

Released 2024-04-11

### Fixed

* Fix `bindgen!` with `trappable_error_type` using unversioned/versioned
  packages.
  [#8305](https://github.com/bytecodealliance/wasmtime/pull/8305)

* cranelift: Include clobbers and outgoing args in stack limit.
  [#8301](https://github.com/bytecodealliance/wasmtime/pull/8301)

* Fix a panic when compiling invalid components.
  [#8323](https://github.com/bytecodealliance/wasmtime/issues/8323)

--------------------------------------------------------------------------------

## 18.0.3

Released 2024-03-12

### Fixed
* Fix inferring native flags when a compilation target is specified.
  [#7991](https://github.com/bytecodealliance/wasmtime/pull/7991)

--------------------------------------------------------------------------------

## 18.0.2

Released 2024-02-28.

### Fixed

* Fix an egraph rule bug that was permitting an incorrect `ireduce` rewrite to
  unary and binary operations, leading to miscompilations.
  [#8005](https://github.com/bytecodealliance/wasmtime/pull/8005)

--------------------------------------------------------------------------------

## 18.0.1

Released 2024-02-20.

### Fixed

* Fixed a mistake in the CI release process that caused the crates.io
  publication of the 18.0.0 release to not succeed.
  [#7966](https://github.com/bytecodealliance/wasmtime/pull/7966)

--------------------------------------------------------------------------------

## 18.0.0

Released 2024-02-20

### Added

* The `wasmtime-c-api-impl` crate is now published on crates.io.
  [#7837](https://github.com/bytecodealliance/wasmtime/pull/7837)

* A new `EngineWeak` type enables holding a weak pointer to an engine with the
  ability to dynamically and fallibly upgrade it to an `Engine`.
  [#7797](https://github.com/bytecodealliance/wasmtime/pull/7797)

* The WebAssembly tail calls proposal can now be enabled through the C API.
  [#7811](https://github.com/bytecodealliance/wasmtime/pull/7811)

* The import and export types of a `Component` can now be inspected at runtime.
  [#7804](https://github.com/bytecodealliance/wasmtime/pull/7804)

* New APIs/types have been added to support a faster version of looking up
  module exports without using string lookups with `Module::get_export_index`.
  [#7828](https://github.com/bytecodealliance/wasmtime/pull/7828)

### Changed

* Owned resources represented with `ResourceAny` can now be passed as arguments
  to functions that require a `borrow<T>` parameter.
  [#7783](https://github.com/bytecodealliance/wasmtime/pull/7783)

* Generated structures from `wasmtime::component::bindgen!` for exported
  interfaces are now all renamed to `Guest` to avoid conflicting with WIT names.
  [#7794](https://github.com/bytecodealliance/wasmtime/pull/7794)

* Guest profiler output will now automatically demangle symbols.
  [#7809](https://github.com/bytecodealliance/wasmtime/pull/7809)

* The `wasmtime` crate now has a `runtime` Cargo feature which, if disabled,
  enables building Wasmtime with only the ability to compile WebAssembly
  modules. This enables compiling Wasmtime's compilation infrastructure, for
  example, to WebAssembly itself.
  [#7766](https://github.com/bytecodealliance/wasmtime/pull/7766)

* Support for the old `wasi-common` crate and the original implementation of
  "WASIp1" aka "preview1" is being deprecated in the `wasmtime-wasi` crate.
  Users should migrate to the  `wasmtime_wasi::preview2` implementation, which
  supports both WASIp1 and WASIp2, as in the next release the
  `wasi-common`-based reexports of `wasmtime-wasi` will be deleted.
  [#7881](https://github.com/bytecodealliance/wasmtime/pull/7881)

--------------------------------------------------------------------------------

## 17.0.3

Released 2024-04-11

### Fixed

* cranelift: Include clobbers and outgoing args in stack limit.
  [#8301](https://github.com/bytecodealliance/wasmtime/pull/8301)

* Fix a panic when compiling invalid components.
  [#8323](https://github.com/bytecodealliance/wasmtime/issues/8323)

--------------------------------------------------------------------------------

## 17.0.2

Released 2024-02-28

### Fixed

* Fix an egraph rule bug that was permitting an incorrect `ireduce` rewrite to
  unary and binary operations, leading to miscompilations.
  [#8005](https://github.com/bytecodealliance/wasmtime/pull/8005)

--------------------------------------------------------------------------------

## 17.0.1

Released 2024-02-07

### Fixed

* Fix an egraph elaboration fuzzbug that was allowing values with dependencies
  that shouldn't be duplicated to be chosen in a context that would make them
  invalid.
  [#7859](https://github.com/bytecodealliance/wasmtime/pull/7859)
  [#7879](https://github.com/bytecodealliance/wasmtime/pull/7879)
* Fix an egraph rule bug that was allowing unconstrained recursion through the
  DFG to run away on large functions.
  [#7882](https://github.com/bytecodealliance/wasmtime/pull/7882)

--------------------------------------------------------------------------------

## 17.0.0

Released 2024-01-25

The major feature of this release is that the WASI support in Wasmtime is now
considered stable and flagged at an 0.2.0 version approved by the WASI subgroup.
The release was delayed a few days to hold off until the WASI subgroup voted to
approve the CLI and HTTP worlds and they're now on-by-default! Additionally the
component model is now enabled by default in Wasmtime, for example an opt-in
flag is no longer required on the CLI. Note that embeddings still must opt-in to
using the component model by using the `wasmtime::component` module.

### Added

* Cranelift optimizations have been added for "3-way comparisons", or `Ord::cmp`
  in Rust or `<=>` in C++.
  [#7636](https://github.com/bytecodealliance/wasmtime/pull/7636)
  [#7702](https://github.com/bytecodealliance/wasmtime/pull/7702)

* Components now use Wasmtime's compilation cache used for core wasm modules by
  default.
  [#7649](https://github.com/bytecodealliance/wasmtime/pull/7649)

* The `Resource<T>` and `ResourceAny` types can now be converted between each
  other.
  [#7649](https://github.com/bytecodealliance/wasmtime/pull/7649)
  [#7712](https://github.com/bytecodealliance/wasmtime/pull/7712)

### Changed

* Minor changes have been made to a number of WITs as they progressed to their
  official 0.2.0 status.
  [#7625](https://github.com/bytecodealliance/wasmtime/pull/7625)
  [#7640](https://github.com/bytecodealliance/wasmtime/pull/7640)
  [#7690](https://github.com/bytecodealliance/wasmtime/pull/7690)
  [#7781](https://github.com/bytecodealliance/wasmtime/pull/7781)
  [#7817](https://github.com/bytecodealliance/wasmtime/pull/7817)

* The component model is now enabled by default.
  [#7821](https://github.com/bytecodealliance/wasmtime/pull/7821)

* The implementation of `memory.atomic.{wait,notify}` has been rewritten.
  [#7629](https://github.com/bytecodealliance/wasmtime/pull/7629)

* The `wasmtime_wasi::preview2::Table` type has been moved to
  `wasmtime::component::ResourceTable`.
  [#7655](https://github.com/bytecodealliance/wasmtime/pull/7655)

* Creating a UDP stream now validates the address being sent to.
  [#7648](https://github.com/bytecodealliance/wasmtime/pull/7648)

* Defining resource types in `Linker<T>` now takes the type to define as a
  runtime parameter.
  [#7680](https://github.com/bytecodealliance/wasmtime/pull/7680)

* Socket address checks can now be performed dynamically at runtime.
  [#7662](https://github.com/bytecodealliance/wasmtime/pull/7662)

* Wasmtime and Cranelift's MSRV is now 1.73.0.
  [#7739](https://github.com/bytecodealliance/wasmtime/pull/7739)

### Fixed

* Bindings for WIT APIs where interfaces have multiple versions are now fixed by
  putting the version in the generated binding names.
  [#7656](https://github.com/bytecodealliance/wasmtime/pull/7656)

* The preview1 `fd_{read,write}` APIs are now fixed when a shared memory is
  used.
  [#7755](https://github.com/bytecodealliance/wasmtime/pull/7755)

* The preview1 `fd_{read,write}` APIs no longer leak an intermediate stream
  created.
  [#7819](https://github.com/bytecodealliance/wasmtime/pull/7819)

--------------------------------------------------------------------------------

## 16.0.0

Released 2023-12-20

### Added

* Add yielding support in `wasmtime_store_epoch_deadline_callback` in the C API.
  [#7476](https://github.com/bytecodealliance/wasmtime/pull/7476)

* Support for the `wasi_unstable` module ("wasi preview0" canonically) has been
  added to the `-Spreview2` support in the CLI.
  [#7548](https://github.com/bytecodealliance/wasmtime/pull/7548)

* The original module can now be obtained from an "instance pre" in the C API.
  [#7572](https://github.com/bytecodealliance/wasmtime/pull/7572)

* Usage of Mach ports on macOS can now be disabled in the C API.
  [#7595](https://github.com/bytecodealliance/wasmtime/pull/7595)

### Changed

* The preview1-to-preview2 component adapters now import a smaller number of
  interfaces by default.
  [#7543](https://github.com/bytecodealliance/wasmtime/pull/7543)
  [#7544](https://github.com/bytecodealliance/wasmtime/pull/7544)

* Wasmtime and Cranelift now require Rust 1.72.0 to build.
  [#7554](https://github.com/bytecodealliance/wasmtime/pull/7554)

* The default `world` supported by `wasmtime serve` has been slimmed down to
  exactly what `wasi:http/proxy` specifies by default. Support for other WASI
  APIs can be included with the `-S common` command-line flag.
  [#7597](https://github.com/bytecodealliance/wasmtime/pull/7597)

* The `wasmtime --version` CLI output will now include date/commit information
  when Wasmtime is built from a git checkout.
  [#7610](https://github.com/bytecodealliance/wasmtime/pull/7610)

* Debug intrinsic symbols required by LLDB and GDB have been moved behind a
  `debug-builtins` feature of the `wasmtime` crate which is enabled by default.
  [#7626](https://github.com/bytecodealliance/wasmtime/pull/7626)

### Fixed

* MPK support is now explicitly disabled on AMD-based CPUs since the
  implementation does not currently support it.
  [#7513](https://github.com/bytecodealliance/wasmtime/pull/7513)

* Initialization of a WebAssembly data segment with a negative offset is fixed
  to zero-extend the offset instead of sign-extend.
  [#7559](https://github.com/bytecodealliance/wasmtime/pull/7559)

* The reported offset of `O_APPEND` files in preview1 has been fixed.
  [#7586](https://github.com/bytecodealliance/wasmtime/pull/7586)

* MPK support does a better job of compacting memories to minimize virtual
  memory used.
  [#7622](https://github.com/bytecodealliance/wasmtime/pull/7622)

### Cranelift

* Union node bitpacking has been fixed with egraph optimizations to ensure the
  minimal cost node is selected.
  [#7465](https://github.com/bytecodealliance/wasmtime/pull/7465)

* Equivalent-cost expressions now have ties broken with expression depth in
  egraphs to prefer "shallow" expression trees.
  [#7456](https://github.com/bytecodealliance/wasmtime/pull/7456)

* Long-and-narrow chains of expressions are now optimized into shallow-and-wide
  trees.
  [#7466](https://github.com/bytecodealliance/wasmtime/pull/7466)

--------------------------------------------------------------------------------

## 15.0.1

Released 2023-12-01.

### Fixed

* The `wasi:random/insecure{,_seed}` interfaces are now available through the
  CLI.
  [#7614](https://github.com/bytecodealliance/wasmtime/pull/7614)

* A stray debugging `println!` was removed.
  [#7618](https://github.com/bytecodealliance/wasmtime/pull/7618)

--------------------------------------------------------------------------------

## 15.0.0

Released 2023-11-20

### Added

* Multiple versions of interfaces are now supported in `bindgen!`.
  [#7172](https://github.com/bytecodealliance/wasmtime/pull/7172)

* UDP has been implemented in `wasi:sockets`.
  [#7148](https://github.com/bytecodealliance/wasmtime/pull/7148)
  [#7243](https://github.com/bytecodealliance/wasmtime/pull/7243)

* Support for custom stack memory allocation has been added.
  [#7209](https://github.com/bytecodealliance/wasmtime/pull/7209)

* The `memory_init_cow` setting can now be configured in the C API.
  [#7227](https://github.com/bytecodealliance/wasmtime/pull/7227)

* The `splice` method of WASI streams has been implemented.
  [#7234](https://github.com/bytecodealliance/wasmtime/pull/7234)

* Wasmtime binary releases now have a `wasmtime-min` executable in addition to
  `libwasmtime-min.*` libraries for the C API. These showcase a minimal
  build of Wasmtime for comparison.
  [#7282](https://github.com/bytecodealliance/wasmtime/pull/7282)
  [#7315](https://github.com/bytecodealliance/wasmtime/pull/7315)
  [#7350](https://github.com/bytecodealliance/wasmtime/pull/7350)

### Changed

* Many changes to `wasi:http` WITs have happened to keep up with the proposal as
  it prepares to reach a more stable status.
  [#7161](https://github.com/bytecodealliance/wasmtime/pull/7161)
  [#7406](https://github.com/bytecodealliance/wasmtime/pull/7406)
  [#7383](https://github.com/bytecodealliance/wasmtime/pull/7383)
  [#7417](https://github.com/bytecodealliance/wasmtime/pull/7417)
  [#7451](https://github.com/bytecodealliance/wasmtime/pull/7451)

* Add an error resource to WASI streams.
  [#7152](https://github.com/bytecodealliance/wasmtime/pull/7152)

* Syntax in `bindgen!`'s `trappable_error_type` configuration has changed.
  [#7170](https://github.com/bytecodealliance/wasmtime/pull/7170)

* TCP errors in `wasi:sockets` have been simplified and clarified.
  [#7120](https://github.com/bytecodealliance/wasmtime/pull/7120)

* Wasmtime/Cranelift now require Rust 1.71.0 to compile.
  [#7206](https://github.com/bytecodealliance/wasmtime/pull/7206)

* Logging in Wasmtime is now configured with `WASMTIME_LOG` instead of
  `RUST_LOG`.
  [#7239](https://github.com/bytecodealliance/wasmtime/pull/7239)

* Fuel-related APIs on `Store` have been refactored and reimplemented with two
  new methods `set_fuel` and `reset_fuel`. Previous methods have been removed.
  [#7240](https://github.com/bytecodealliance/wasmtime/pull/7240)
  [#7298](https://github.com/bytecodealliance/wasmtime/pull/7298)

* The `forward` method of WASI streams has been removed.
  [#7234](https://github.com/bytecodealliance/wasmtime/pull/7234)

* The WebAssembly `threads`, `multi-memory`, and `relaxed-simd` proposals are
  now enabled by default.
  [#7285](https://github.com/bytecodealliance/wasmtime/pull/7285)

* Logging is now implemented for `wasmtime serve`.
  [#7366](https://github.com/bytecodealliance/wasmtime/pull/7366)

* Filesystem locking has been temporarily removed from WASI.
  [#7355](https://github.com/bytecodealliance/wasmtime/pull/7355)

* Wasmtime's implementation of WASI preview1 built on top of preview2
  (`-Spreview2`) has been enabled by default.
  [#7365](https://github.com/bytecodealliance/wasmtime/pull/7365)

* The `wasi:clocks` interface now has two `subscribe` functions and a `duration`
  type.
  [#7358](https://github.com/bytecodealliance/wasmtime/pull/7358)

* The `wasi:io/poll` interface has seen some refactoring.
  [#7427](https://github.com/bytecodealliance/wasmtime/pull/7427)

### Fixed

* Profiling the first function in a module now works.
  [#7254](https://github.com/bytecodealliance/wasmtime/pull/7254)

* Consecutive writes to files in preview2 have been fixed.
  [#7394](https://github.com/bytecodealliance/wasmtime/pull/7394)

* Copy-on-write initialization of linear memories has been fixed for components.
  [#7459](https://github.com/bytecodealliance/wasmtime/pull/7459)

### Cranelift

* Support for proof-carrying code has been added to Cranelift to assist with an
  extra layer of validation about properties such as WebAssembly memory accesses
  in the future.
  [#7165](https://github.com/bytecodealliance/wasmtime/pull/7165)
  [#7180](https://github.com/bytecodealliance/wasmtime/pull/7180)
  [#7219](https://github.com/bytecodealliance/wasmtime/pull/7219)
  [#7231](https://github.com/bytecodealliance/wasmtime/pull/7231)
  [#7262](https://github.com/bytecodealliance/wasmtime/pull/7262)
  [#7263](https://github.com/bytecodealliance/wasmtime/pull/7263)
  [#7274](https://github.com/bytecodealliance/wasmtime/pull/7274)
  [#7280](https://github.com/bytecodealliance/wasmtime/pull/7280)
  [#7281](https://github.com/bytecodealliance/wasmtime/pull/7281)
  [#7352](https://github.com/bytecodealliance/wasmtime/pull/7352)
  [#7389](https://github.com/bytecodealliance/wasmtime/pull/7389)
  [#7468](https://github.com/bytecodealliance/wasmtime/pull/7468)

* Rematerialization of values no longer accidentally overrides LICM.
  [#7306](https://github.com/bytecodealliance/wasmtime/pull/7306)

* Inline stack probes no longer make Valgrind unhappy.
  [#7470](https://github.com/bytecodealliance/wasmtime/pull/7470)

--------------------------------------------------------------------------------

## 14.0.4

Released 2023-11-01

### Fixed

* Using the `--dir` argument combined with a `::`-remapped path no longer prints
  a warning about compatibility with the old CLI and works with remapping.
  [#7416](https://github.com/bytecodealliance/wasmtime/pull/7416)

* Consecutive file writes in preview2 have been fixed.
  [#7394](https://github.com/bytecodealliance/wasmtime/pull/7394)

--------------------------------------------------------------------------------

## 14.0.3

Released 2023-10-29

### Fixed

* The `wasmtime` executable will now attempt to more gracefully handle the
  transition from the 13.0.0 CLI arguments and parsing to the changes in 14.0.0.
  CLI commands should now warn if they no longer work with the new parser, but
  still execute as they previously did. This behavior can be controlled via a
  new `WASMTIME_NEW_CLI` environment variable if necessary.
  [#7385](https://github.com/bytecodealliance/wasmtime/pull/7385)

* The `serve` subcommand of the `wasmtime` CLI is now enabled by default for the
  `wasmtime` executable.
  [#7392](https://github.com/bytecodealliance/wasmtime/pull/7392)

--------------------------------------------------------------------------------

## 14.0.2

Released 2023-10-26

### Fixed

* Make the `wasmtime::unix` module accessible on macOS again.
  [#7360](https://github.com/bytecodealliance/wasmtime/pull/7360)

* Inter-crate dependencies between `cranelift-*` crates now disable the
  `default` feature meaning that it's possible for embedders to depend on
  `cranelift-codegen` as well without the `default` feature.
  [#7369](https://github.com/bytecodealliance/wasmtime/pull/7369)

--------------------------------------------------------------------------------

## 14.0.1

Released 2023-10-23

### Fixed

* Cranelift: preserve uext and sext flags for parameters on x86\_64 and apple
  aarch64. Note that this does not affect Wasmtime and is only intended for
  Cranelift embedders such as `rustc_codegen_cranelift`.
  [#7333](https://github.com/bytecodealliance/wasmtime/pull/7333)

--------------------------------------------------------------------------------

## 14.0.0

Released 2023-10-20

One of the larger changes in this release is a redesign of Wasmtime's CLI
arguments and where arguments are passed. This means that previous invocations
of the `wasmtime` CLI executable will need to be updated. No functionality was
removed but most of it is behind new flags. One major change is that Wasmtime
CLI flags are now grouped behind short options like `-O`. For example

    wasmtime run --opt-level 2 foo.wasm

is now:

    wasmtime run -O opt-level=2 foo.wasm

Additionally options prefixed with `--enable-*` or `--disable-*` now
consistently are considered boolean setters. For example:

    wasmtime run --disable-cache foo.wasm

is now:

    wasmtime run -C cache=n foo.wasm

Options can be explored with `wasmtime -C help` for example, and `wasmtime -h`
will show all option groups that can be expanded.

Another major change in the CLI is that any CLI argument which positionally
comes after the wasm file specified will be passed as an argument to the guest
module. For example this invocations

    wasmtime run foo.wasm --epoch-interruption

was previously accepted as enabling epoch interruption for the `foo.wasm` file.
This is now interpreted as if it were `./foo.wasm --epoch-interruption`,
however, passing the flag to the wasm file itself. Flags to Wasmtime must now
come after Wasmtime's subcommand (in this case `run`) and before the wasm file
that's being run, for example:

    wasmtime run -W epoch-interruption foo.wasm

More information about this change can be found on
[#6925](https://github.com/bytecodealliance/wasmtime/pull/6925) and
[#6946](https://github.com/bytecodealliance/wasmtime/pull/6946).

### Added

* Added the `wasmtime::FrameInfo::module` method, which returns the
  `wasmtime::Module` associated with the stack frame.

* The `wasmtime::component::Linker` type now implements `Clone`.
  [#7032](https://github.com/bytecodealliance/wasmtime/pull/7032)

* Wasmtime's `TypedFunc` API now supports the `v128` WebAssembly type on x86\_64
  and aarch64.
  [#7010](https://github.com/bytecodealliance/wasmtime/pull/7010)

* Support for resources exported from a WebAssembly guest has been added to the
  component `bindgen!` macro.
  [#7050](https://github.com/bytecodealliance/wasmtime/pull/7050)

* The C API now supports learning about a module's `image_range`.
  [#7064](https://github.com/bytecodealliance/wasmtime/pull/7064)

* Passing values between components is now possible with a more complete
  implementation of type-checking of values.
  [#7065](https://github.com/bytecodealliance/wasmtime/pull/7065)

* Types representing resources can now be customized with `bindgen!`.
  [#7069](https://github.com/bytecodealliance/wasmtime/pull/7069)

* Wasm-defined globals and memories are now included in core dumps, and the
  `wasmtime::WasmCoreDump` type is now serializable.
  [#6935](https://github.com/bytecodealliance/wasmtime/pull/6935)
  [#7078](https://github.com/bytecodealliance/wasmtime/pull/7078)

* Initial experimental support for Intel MPK has been added to support running
  more instances concurrently.
  [#7072](https://github.com/bytecodealliance/wasmtime/pull/7072)

* The implementation of `wasi:http` now supports inbound requests in addition to
  outbound requests. A new `wasmtime serve` command is an example way of
  handling http requests with wasm files.
  [#7091](https://github.com/bytecodealliance/wasmtime/pull/7091)

* The C API now supports Wasmtime's "host memory creation" API to customize the
  allocation of linear memories.
  [#7115](https://github.com/bytecodealliance/wasmtime/pull/7115)

* The C API now supports asynchronous invocation of WebAssembly programs.
  [#7106](https://github.com/bytecodealliance/wasmtime/pull/7106)

* The C API now supports Wasmtime's `InstancePre<T>` type.
  [#7140](https://github.com/bytecodealliance/wasmtime/pull/7140)

* The `wasi:sockets/ip-name-lookup` interface is now implemented by Wasmtime.
  [#7109](https://github.com/bytecodealliance/wasmtime/pull/7109)

### Changed

* Wasmtime's CLI has been significantly overhauled. See the note above.
  [#6925](https://github.com/bytecodealliance/wasmtime/pull/6925)
  [#6946](https://github.com/bytecodealliance/wasmtime/pull/6946)

* The `wasmtime::FrameInfo::module_name` has been removed, however you can now
  get identical results by chaining `wasmtime::FrameInfo::module` and
  `wasmtime::Module::name`: `my_frame.module().name()`.

* WASI interfaces have seen significant work since the previous release. Streams
  for example have a new backpressure and flushing design. Additionally WIT
  `resource`s are now used ubiquitously throughout the specification and
  implementation.
  [#6877](https://github.com/bytecodealliance/wasmtime/pull/6877)
  [#7029](https://github.com/bytecodealliance/wasmtime/pull/7029)
  [#7090](https://github.com/bytecodealliance/wasmtime/pull/7090)

* The implementation of `wasi:http` now uses `{input,output}-stream` from the
  `wasi:io/streams` interface.
  [#7056](https://github.com/bytecodealliance/wasmtime/pull/7056)

* Lifting and lowering of the `list<u8>` component values has been significantly
  optimized.
  [#6971](https://github.com/bytecodealliance/wasmtime/pull/6971)

* The `wasmtime-c-api` crate is now additionally built as an rlib as well as the
  previous cdylib/staticlib combo.
  [#6765](https://github.com/bytecodealliance/wasmtime/pull/6765)

### Fixed

* Support referencing stack slots in the DWARF debug info.
  [#6960](https://github.com/bytecodealliance/wasmtime/pull/6960)

* Printing unicode to stdio on Windows has been fixed.
  [#6825](https://github.com/bytecodealliance/wasmtime/pull/6825)

* Building for x86\_64-linux-android has been fixed.
  [#7055](https://github.com/bytecodealliance/wasmtime/pull/7055)

* Fixed stdout/stderr becoming nonblocking by accident with WASI preview2 on
  macOS.
  [#7058](https://github.com/bytecodealliance/wasmtime/pull/7058)

* Fixed some character boundary-related panics in the preview2 implementation of
  preview1.
  [#7011](https://github.com/bytecodealliance/wasmtime/pull/7011)

* Fixed an issue of guests sleeping for an incorrect amount of time with
  preview2.
  [#6993](https://github.com/bytecodealliance/wasmtime/pull/6993)

* Cranelift will now return an error when running out of temporaries in a very
  large function instead of panicking.
  [#7114](https://github.com/bytecodealliance/wasmtime/pull/7114)

--------------------------------------------------------------------------------

## 13.0.1

Released 2023-10-26

### Fixed

* Make the `wasmtime::unix` module accessible on macOS again.
  [#7360](https://github.com/bytecodealliance/wasmtime/pull/7360)

--------------------------------------------------------------------------------

## 13.0.0

Released 2023-09-20

### Added

* Configuration of mach ports vs signals on macOS is now done through a `Config`
  instead of at compile time.
  [#6807](https://github.com/bytecodealliance/wasmtime/pull/6807)

* `Engine::detect_precompiled{,_file}` can be used to determine whether some
  bytes or a file look like a precompiled module or a component.
  [#6832](https://github.com/bytecodealliance/wasmtime/pull/6832)
  [#6937](https://github.com/bytecodealliance/wasmtime/pull/6937)

* A new feature "wmemcheck" has been added to enable Valgrind-like detection of
  use-after-free within a WebAssembly guest module.
  [#6820](https://github.com/bytecodealliance/wasmtime/pull/6820)
  [#6856](https://github.com/bytecodealliance/wasmtime/pull/6856)

* The `wasmtime` CLI now supports executing components.
  [#6836](https://github.com/bytecodealliance/wasmtime/pull/6836)

* Support for WASI preview2's TCP sockets interface has been added.
  [#6837](https://github.com/bytecodealliance/wasmtime/pull/6837)

* Wasmtime's implementation of the wasi-nn proposal now supports named models.
  [#6854](https://github.com/bytecodealliance/wasmtime/pull/6854)

* The C API now supports configuring `native_unwind_info`,
  `dynamic_memory_reserved_for_growth`, `target`, and Cranelift settings.
  [#6896](https://github.com/bytecodealliance/wasmtime/pull/6896)
  [#6934](https://github.com/bytecodealliance/wasmtime/pull/6934)

* The `wasmtime` crate now has initial support for component model bindings
  generation for the WIT `resource` type.
  [#6886](https://github.com/bytecodealliance/wasmtime/pull/6886)

* Cranelift's RISC-V backend now has a complete implementation of the
  WebAssembly SIMD proposal. Many thanks to Afonso Bordado for all their
  contributions!
  [#6920](https://github.com/bytecodealliance/wasmtime/pull/6920)
  [#6924](https://github.com/bytecodealliance/wasmtime/pull/6924)

* The `bindgen!` macro in the `wasmtime` crate now supports conditional
  configuration for which imports should be `async` and which should be
  synchronous.
  [#6942](https://github.com/bytecodealliance/wasmtime/pull/6942)

### Changed

* The pooling allocator was significantly refactored and the
  `PoolingAllocationConfig` has some minor breaking API changes that reflect
  those changes.

  Previously, the pooling allocator had `count` slots, and each slot had `N`
  memories and `M` tables. Every allocated instance would reserve those `N`
  memories and `M` tables regardless whether it actually needed them all or
  not. This could lead to some waste and over-allocation when a module used less
  memories and tables than the pooling allocator's configured maximums.

  After the refactors in this release, the pooling allocator doesn't have
  one-size-fits-all slots anymore. Instead, memories and tables are in separate
  pools that can be allocated from independently, and we allocate exactly as
  many memories and tables as are necessary for the instance being allocated.

  To preserve your old configuration with the new methods you can do the following:

  ```rust
  let mut config = PoolingAllocationConfig::default();

  // If you used to have this old, no-longer-compiling configuration:
  config.count(count);
  config.instance_memories(n);
  config.instance_tables(m);

  // You can use these equivalent settings for the new config methods:
  config.total_core_instances(count);
  config.total_stacks(count); // If using the `async` feature.
  config.total_memories(count * n);
  config.max_memories_per_module(n);
  config.total_tables(count * m);
  config.max_tables_per_module(m);
  ```

  There are additionally a variety of methods to limit the maximum amount of
  resources a single core Wasm or component instance can take from the pool:

  * `PoolingAllocationConfig::max_memories_per_module`
  * `PoolingAllocationConfig::max_tables_per_module`
  * `PoolingAllocationConfig::max_memories_per_component`
  * `PoolingAllocationConfig::max_tables_per_component`
  * `PoolingAllocationConfig::max_core_instances_per_component`

  These methods do not affect the size of the pre-allocated pool.
  [#6835](https://github.com/bytecodealliance/wasmtime/pull/6835)

* Builder methods for WASI contexts now use `&mut self` instead of `self`.
  [#6770](https://github.com/bytecodealliance/wasmtime/pull/6770)

* Native unwinding information is now properly disabled when it is configured to
  be turned off.
  [#6547](https://github.com/bytecodealliance/wasmtime/pull/6547)

* Wasmtime's minimum supported Rust version (MSRV) is now 1.70.0. Wasmtime's
  MSRV policy of supporting the last three releases of Rust (N-2) is now
  additionally documented. More discussion can additionally be found on the PR
  itself.
  [#6900](https://github.com/bytecodealliance/wasmtime/pull/6900)

* Wasmtime's support for DWARF debugging information has seen some fixes for
  previously reported crashes.
  [#6931](https://github.com/bytecodealliance/wasmtime/pull/6931)

### Removed

* Wasmtime's experimental implementation of wasi-crypto has been removed. More
  discussion of this change can be found on
  [#6732](https://github.com/bytecodealliance/wasmtime/pull/6732)
  and
  [#6816](https://github.com/bytecodealliance/wasmtime/pull/6816)

* Support for `union` types in the component model has been removed.
  [#6913](https://github.com/bytecodealliance/wasmtime/pull/6913)

--------------------------------------------------------------------------------

## 12.0.2

Released 2023-09-14.

### Fixed

* [CVE-2023-41880] - Miscompilation of wasm `i64x2.shr_s` instruction with
  constant input on x86\_64

[CVE-2023-41880]: https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-gw5p-q8mj-p7gh

--------------------------------------------------------------------------------

## 12.0.1

Released 2023-08-24

### Fixed

* Optimized the cranelift compilation on aarch64 for large wasm modules.
  [#6804](https://github.com/bytecodealliance/wasmtime/pull/6804)

--------------------------------------------------------------------------------

## 12.0.0

Released 2023-08-21

### Added

* Wasmtime now supports having multiple different versions of itself being
  linked into the same final executable by mangling some C symbols used by
  Wasmtime.
  [#6673](https://github.com/bytecodealliance/wasmtime/pull/6673)

* The `perfmap` profiling option is now supported on any Unix platform instead
  of just Linux.
  [#6701](https://github.com/bytecodealliance/wasmtime/pull/6701)

* The `wasmtime` CLI now supports `--env FOO` to inherit the value of the
  environment variable `FOO` which avoids needing to do `--env FOO=$FOO` for
  example.
  [#6746](https://github.com/bytecodealliance/wasmtime/pull/6746)

* Wasmtime now supports component model resources, although support has not yet
  been added to `bindgen!`.
  [#6691](https://github.com/bytecodealliance/wasmtime/pull/6691)

* Wasmtime now supports configuration to enable the tail calls proposal.
  Platform support now also includes AArch64 and RISC-V in addition to the
  previous x86\_64 support.
  [#6723](https://github.com/bytecodealliance/wasmtime/pull/6723)
  [#6749](https://github.com/bytecodealliance/wasmtime/pull/6749)
  [#6774](https://github.com/bytecodealliance/wasmtime/pull/6774)

* Wasmtime's implementation of WASI Preview 2 now supports streams/pollables
  with host objects that are all backed by Rust `async`.
  [#6556](https://github.com/bytecodealliance/wasmtime/pull/6556)

* Support for core dumps has now been added to the `wasmtime` crate.
  [#6513](https://github.com/bytecodealliance/wasmtime/pull/6513)

* New `{Module,Component}::resources_required` APIs allow inspecting what will
  be required when instantiating the module or component.
  [#6789](https://github.com/bytecodealliance/wasmtime/pull/6789)

### Fixed

* Functions on instances defined through `component::Linker::func_new` are now
  defined correctly.
  [#6637](https://github.com/bytecodealliance/wasmtime/pull/6637)

* The `async_stack_size` configuration option is no longer inspected when
  `async_support` is disabled at runtime.
  [#6771](https://github.com/bytecodealliance/wasmtime/pull/6771)

* WASI Preview 1 APIs will now trap on misaligned or out-of-bounds pointers
  instead of returning an error.
  [#6776](https://github.com/bytecodealliance/wasmtime/pull/6776)

### Changed

* Empty types are no longer allowed in the component model.
  [#6777](https://github.com/bytecodealliance/wasmtime/pull/6777)

--------------------------------------------------------------------------------

## 11.0.2

Released 2023-09-14.

### Fixed

* [CVE-2023-41880] - Miscompilation of wasm `i64x2.shr_s` instruction with
  constant input on x86\_64

[CVE-2023-41880]: https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-gw5p-q8mj-p7gh

--------------------------------------------------------------------------------

## 11.0.1

Released 2023-07-24.

### Fixed

* Update some minimum version requirements for Wasmtime's dependencies to fix
  building Wasmtime with historical versions of these dependencies.
  [#6758](https://github.com/bytecodealliance/wasmtime/pull/6758)

--------------------------------------------------------------------------------

## 11.0.0

Released 2023-07-20

### Changed

* The WASI Preview 2 `WasiCtxBuilder` type has been refactored, and `WasiCtx` now has private
  fields.
  [#6652](https://github.com/bytecodealliance/wasmtime/pull/6652)

* Component `bindgen!` now generates owned types by default instead of based on
  how they're used
  [#6648](https://github.com/bytecodealliance/wasmtime/pull/6648)

* Wasmtime/Cranelift on x86-64 can now execute Wasm-SIMD on baseline SSE2, which
  all x86-64 processors support (as part of the base x86-64 spec). Previously,
  SSE4.2 extensions were required. This new work allows Wasm with SIMD
  extensions to execute on processors produced back to 2003.
  [#6625](https://github.com/bytecodealliance/wasmtime/pull/6625)


### Fixed

* Only export the top-level preview2 module from wasmtime-wasi when the
  `preview2` feature is enabled.
  [#6615](https://github.com/bytecodealliance/wasmtime/pull/6615)


### Cranelift changes

* Tail call implementation has begun in Cranelift
  [#6641](https://github.com/bytecodealliance/wasmtime/pull/6641)
  [#6666](https://github.com/bytecodealliance/wasmtime/pull/6666)
  [#6650](https://github.com/bytecodealliance/wasmtime/pull/6650)
  [#6635](https://github.com/bytecodealliance/wasmtime/pull/6635)
  [#6608](https://github.com/bytecodealliance/wasmtime/pull/6608)
  [#6586](https://github.com/bytecodealliance/wasmtime/pull/6586)

* Work continues on SIMD support for the riscv64 backend
  [#6657](https://github.com/bytecodealliance/wasmtime/pull/6657)
  [#6643](https://github.com/bytecodealliance/wasmtime/pull/6643)
  [#6601](https://github.com/bytecodealliance/wasmtime/pull/6601)
  [#6609](https://github.com/bytecodealliance/wasmtime/pull/6609)
  [#6602](https://github.com/bytecodealliance/wasmtime/pull/6602)
  [#6598](https://github.com/bytecodealliance/wasmtime/pull/6598)
  [#6599](https://github.com/bytecodealliance/wasmtime/pull/6599)
  [#6587](https://github.com/bytecodealliance/wasmtime/pull/6587)
  [#6568](https://github.com/bytecodealliance/wasmtime/pull/6568)
  [#6515](https://github.com/bytecodealliance/wasmtime/pull/6515)

* Fix `AuthenticatedRet` when stack bytes are popped in the aarch64 backend
  [#6634](https://github.com/bytecodealliance/wasmtime/pull/6634)

* The `fcvt_low_from_sint` instruction has been removed, as it its current
  behavior can be recovered through a combination of `swiden_low` and
  `fcvt_from_sint`
  [#6565](https://github.com/bytecodealliance/wasmtime/pull/6565)

--------------------------------------------------------------------------------

## 10.0.2

Released 2023-09-14.

### Fixed

* [CVE-2023-41880] - Miscompilation of wasm `i64x2.shr_s` instruction with
  constant input on x86\_64

[CVE-2023-41880]: https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-gw5p-q8mj-p7gh

--------------------------------------------------------------------------------

## 10.0.1

Released 2023-06-21

### Fixed

* Only export the top-level preview2 module from wasmtime-wasi when the
  `preview2` feature is enabled.
  [#6615](https://github.com/bytecodealliance/wasmtime/pull/6615)

--------------------------------------------------------------------------------

## 10.0.0

Released 2023-06-20

### Added

* Expose the `Config::static_memory_forced` option through the C api
  [#6413](https://github.com/bytecodealliance/wasmtime/pull/6413)

* Basic guest-profiler documentation for the book
  [#6394](https://github.com/bytecodealliance/wasmtime/pull/6394)

* Merge the initial wasi-preview2 implementation
  [#6391](https://github.com/bytecodealliance/wasmtime/pull/6391)

* The wasi-preview2 component adapter has been pulled into the main wasmtime
  repository. It is available for the first time as part of this release, but should be
  treated as as a beta at this time. Patch releases will not be made for bug fixes.
  [#6374](https://github.com/bytecodealliance/wasmtime/pull/6374)

* A callback invoked when an epoch deadline is reached can now be configured via
  the C API.
  [#6359](https://github.com/bytecodealliance/wasmtime/pull/6359)

* PR auto-assignment policies have been documented, to clarify the expectations of
  reviewers.
  [#6346](https://github.com/bytecodealliance/wasmtime/pull/6346)

* Support for the function references has been added
  [#5288](https://github.com/bytecodealliance/wasmtime/pull/5288)

### Changed

* An `epoch_deadline_callback` now returns an `UpdateDeadline` enum to allow
  optionally yielding to the async executor after the callback runs.
  [#6464](https://github.com/bytecodealliance/wasmtime/pull/6464)

* The `--profile-guest` flag has now been folded into `--profile=guest`
  [#6352](https://github.com/bytecodealliance/wasmtime/pull/6352)

* Initializers are no longer tracked in the type information for globals, and
  instead are provided when creating the global.
  [#6349](https://github.com/bytecodealliance/wasmtime/pull/6349)

* The "raw" representation of `funcref` and `externref` in the embedding API has
  been updated from a `usize` to a `*mut u8` to be compatible with Rust's
  proposed strict provenance rules. This change is additionally reflected into
  the C API as well.
  [#6338](https://github.com/bytecodealliance/wasmtime/pull/6338)

### Fixed

* Fixed a soundness issue with the component model and async
  [#6509](https://github.com/bytecodealliance/wasmtime/pull/6509)

* Opening directories with WASI on Windows with `NONBLOCK` in flags has been
  fixed.
  [#6348](https://github.com/bytecodealliance/wasmtime/pull/6348)

### Cranelift changes

* Performance improvements in regalloc2 have landed, and compilation time has
  improved
  [#6483](https://github.com/bytecodealliance/wasmtime/pull/6483)
  [#6398](https://github.com/bytecodealliance/wasmtime/pull/6398)

* Renamed `abi::Caller` to `abi::CallSite`
  [#6414](https://github.com/bytecodealliance/wasmtime/pull/6414)

* Work has begun on SIMD support for the riscv64 backend
  [#6324](https://github.com/bytecodealliance/wasmtime/pull/6324)
  [#6366](https://github.com/bytecodealliance/wasmtime/pull/6366)
  [#6367](https://github.com/bytecodealliance/wasmtime/pull/6367)
  [#6392](https://github.com/bytecodealliance/wasmtime/pull/6392)
  [#6397](https://github.com/bytecodealliance/wasmtime/pull/6397)
  [#6403](https://github.com/bytecodealliance/wasmtime/pull/6403)
  [#6408](https://github.com/bytecodealliance/wasmtime/pull/6408)
  [#6419](https://github.com/bytecodealliance/wasmtime/pull/6419)
  [#6430](https://github.com/bytecodealliance/wasmtime/pull/6430)
  [#6507](https://github.com/bytecodealliance/wasmtime/pull/6507)

--------------------------------------------------------------------------------

## 9.0.3

Released 2023-05-31.

### Fixed

* Fix Wasi rights system to work with wasi-testsuite, which exposed a corner case
  that was missed by the fixes in the 9.0.2 release.
  [#6479](https://github.com/bytecodealliance/wasmtime/pull/6479)

--------------------------------------------------------------------------------

## 9.0.2

Released 2023-05-26.

### Fixed

* Fix Wasi rights system to work with wasi-libc. This regression was
  introduced in the 9.0.0 release.
  [#6462](https://github.com/bytecodealliance/wasmtime/pull/6462)
  [#6471](https://github.com/bytecodealliance/wasmtime/pull/6471)

--------------------------------------------------------------------------------

## 9.0.1

Released 2023-05-22.

### Fixed

* A panic which happened when enabling support for native platform profilers was
  fixed.
  [#6435](https://github.com/bytecodealliance/wasmtime/pull/6435)

--------------------------------------------------------------------------------

## 9.0.0

Released 2023-05-22.

### Added

* Initial integration of the Winch baseline compiler into Wasmtime is
  implemented. Note that Winch still does not support much of WebAssembly, but
  intrepid explorers may have an easier time playing around with it now.
  [#6119](https://github.com/bytecodealliance/wasmtime/pull/6119)

* The `wasmtime` CLI now has flags to limit memory, instances, and tables. For
  example `--max-memory-size` or `--max-tables`. Additionally it has a new
  `--trap-on-grow-failure` option to force a trap whenever a `memory.grow` would
  otherwise fail which can be useful for debugging modules which may be
  encountering OOM.
  [#6149](https://github.com/bytecodealliance/wasmtime/pull/6149)

* An initial implementation of the wasi-http proposal was added to Wasmtime in
  the shape of a new `wasmtime-wasi-http` crate and a
  `--wasi-modules=experimental-wasi-http` CLI flag.  Note that this is not
  on-by-default and still in an experimental status at this time.
  [#5929](https://github.com/bytecodealliance/wasmtime/pull/5929)

* Wasmtime's `bindgen!` macro for components now has `interfaces` and
  `with` options to configure use of interfaces defined externally in separate
  crates.
  [#6160](https://github.com/bytecodealliance/wasmtime/pull/6160)
  [#6210](https://github.com/bytecodealliance/wasmtime/pull/6210)

* Wasmtime's `bindgen!` macro emits trace events for arguments and results
  when enabled.
  [#6209](https://github.com/bytecodealliance/wasmtime/pull/6209)

* A new `Engine::precompile_compatibility_hash` method has been added to assist
  with hashing artifacts to be compatible with versions of Wasmtime.
  [#5826](https://github.com/bytecodealliance/wasmtime/pull/5826)

* Wasmtime's C API now has functions for enabling the WebAssembly relaxed-simd
  proposal.
  [#6292](https://github.com/bytecodealliance/wasmtime/pull/6292)

* A new `--emit-clif` flag has been added to `wasmtime compile` to see the CLIF
  corresponding to a WebAssembly module to be used for debugging.
  [#6307](https://github.com/bytecodealliance/wasmtime/pull/6307)

* Support for an in-process sampling-based profiler has been added to Wasmtime.
  This is intended to be used in conjunction with epochs to enable relatively
  simple implementations of profiling a guest module.
  [#6282](https://github.com/bytecodealliance/wasmtime/pull/6282)

### Changed

* Overhauled the way that Wasmtime calls into Wasm and Wasm calls back out to
  the host. Instead of chaining together trampolines to convert between calling
  conventions, we now represent `funcref`s with multiple function pointers, one
  per calling convention. This paves the way for supporting Wasm tail calls and
  also results in ~10% speed ups to a variety of function call benchmarks,
  however there are some slight compiled Wasm module code size regressions
  (which can be alleviated by disabling optional `.eh_frame`
  generation). Additionally, in the C API the `wasmtime_func_call_unchecked`
  function gained one more parameter, which is the capacity of the
  args-and-results
  buffer.
  [#6262](https://github.com/bytecodealliance/wasmtime/pull/6262)

* The `wasmtime compile` command will now default to producing executables for
  the native host and its CPU features instead of the baseline feature set of
  the host's architecture.
  [#6152](https://github.com/bytecodealliance/wasmtime/pull/6152)

* The `ResourceLimiter` trait and its `async` equivalent now support returning
  errors from growth to force a trap in the wasm module rather than reporting
  -1 to the wasm module. Note that this is primarily intended for debugging.
  [#6149](https://github.com/bytecodealliance/wasmtime/pull/6149)

* The non-egraph-based optimization pipeline has been removed from Cranelift,
  and the corresponding `Config::use_egraphs` option is also removed.
  [#6167](https://github.com/bytecodealliance/wasmtime/pull/6167)

* Generated types for WIT files now always generates owned types by default.
  [#6189](https://github.com/bytecodealliance/wasmtime/pull/6189)

* Wasmtime's baseline x86\_64 CPU features required for SIMD support has been
  lowered from SSE 4.2 to SSE 4.1.
  [#6206](https://github.com/bytecodealliance/wasmtime/pull/6206)

* The `fd_allocate` implementation in Wasmtime will now always fail with
  `ENOTSUP`.
  [#6217](https://github.com/bytecodealliance/wasmtime/pull/6217)

* The "rights" system in WASI has been removed and rights are no longer
  inspected in the implementation of any WASI functions.
  [#6265](https://github.com/bytecodealliance/wasmtime/pull/6265)

### Fixed

* WASI can now open directories without `O_DIRECTORY`.
  [#6163](https://github.com/bytecodealliance/wasmtime/pull/6163)

* The `poll_oneoff` function has been fixed when handling non-regular files.
  [#6258](https://github.com/bytecodealliance/wasmtime/pull/6258)

* The behavior of `path_readlink` on too-small buffers has been fixed to
  truncate.
  [#6225](https://github.com/bytecodealliance/wasmtime/pull/6225)

### Cranelift changes

> Note: this section documents changes to Cranelift, a code generator backend
> that Wasmtime uses. These changes are not always applicable to Wasmtime as a
> WebAssembly runtime but may be interesting to other projects which embed or
> use Cranelift.

* New `{u,s}{add,sub,mul}_overflow` instructions have been added.
  [#5784](https://github.com/bytecodealliance/wasmtime/pull/5784)

* The `iadd_cout` and `isub_bout` instructions have been removed.
  [#6198](https://github.com/bytecodealliance/wasmtime/pull/6198)

* ISLE now supports binary and octal integer literals.
  [#6234](https://github.com/bytecodealliance/wasmtime/pull/6234)

* An implementation of SIMD for RISC-V has started.
  [#6240](https://github.com/bytecodealliance/wasmtime/pull/6240)
  [#6266](https://github.com/bytecodealliance/wasmtime/pull/6266)
  [#6268](https://github.com/bytecodealliance/wasmtime/pull/6268)

--------------------------------------------------------------------------------

## 8.0.1

Released 2023-04-27.

### Changed

* Breaking: Files opened using Wasmtime's implementation of WASI on Windows now
  cannot be deleted until the file handle is closed. This was already true for
  open directories. The change was necessary for the bug fix in
  [#6163](https://github.com/bytecodealliance/wasmtime/pull/6163).

### Fixed

* Fixed wasi-common's implementation of the `O_DIRECTORY` flag to match POSIX.
  [#6163](https://github.com/bytecodealliance/wasmtime/pull/6163)

* Undefined Behavior in Rust runtime functions
  [GHSA-ch89-5g45-qwc7](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-ch89-5g45-qwc7)

--------------------------------------------------------------------------------

## 8.0.0

Released 2023-04-20

### Added

* Allow the MPL-2.0 and OpenSSL licenses in dependencies of wasmtime.
  [#6136](https://github.com/bytecodealliance/wasmtime/pull/6136)

* Add a bounds-checking optimization for dynamic memories and guard pages.
  [#6031](https://github.com/bytecodealliance/wasmtime/pull/6031)

* Add support for generating perf maps for simple perf profiling. Additionally,
  the `--jitdump` and `--vtune` flags have been replaced with a single
  `--profile` flags that accepts `perfmap`, `jitdump`, and `vtune` arguments.
  [#6030](https://github.com/bytecodealliance/wasmtime/pull/6030)

* Validate faulting addresses are valid to fault on. As a mitigation to CVEs
  like `GHSA-ff4p-7xrq-q5r8`, check that the address involved in a fault is one
  that could be contained in a `Store`, or print a scary message and abort
  immediately.
  [#6028](https://github.com/bytecodealliance/wasmtime/pull/6028)

* Add the `--default-values-unknown-imports` option to define unknown function
  imports as functions that return the default value for their result type.
  [#6010](https://github.com/bytecodealliance/wasmtime/pull/6010)

* Add `Clone` for `component::InstancePre`.
  [#5996](https://github.com/bytecodealliance/wasmtime/issues/5996)

* Add `--dynamic-memory-reserved-for-growth` cli flag.
  [#5980](https://github.com/bytecodealliance/wasmtime/issues/5980)

* Introduce the `wasmtime-explorer` crate for investigating the compilation of
  wasm modules. This functionality is also exposed via the `wasmtime explore`
  command.
  [#5975](https://github.com/bytecodealliance/wasmtime/pull/5975)

* Added support for the Relaxed SIMD proposal.
  [#5892](https://github.com/bytecodealliance/wasmtime/pull/5892)

* Cranelift gained many new machine-independent optimizations.
  [#5909](https://github.com/bytecodealliance/wasmtime/pull/5909)
  [#6032](https://github.com/bytecodealliance/wasmtime/pull/6032)
  [#6033](https://github.com/bytecodealliance/wasmtime/pull/6033)
  [#6034](https://github.com/bytecodealliance/wasmtime/pull/6034)
  [#6037](https://github.com/bytecodealliance/wasmtime/pull/6037)
  [#6052](https://github.com/bytecodealliance/wasmtime/pull/6052)
  [#6053](https://github.com/bytecodealliance/wasmtime/pull/6053)
  [#6072](https://github.com/bytecodealliance/wasmtime/pull/6072)
  [#6095](https://github.com/bytecodealliance/wasmtime/pull/6095)
  [#6130](https://github.com/bytecodealliance/wasmtime/pull/6130)

### Changed

* Derive `Copy` on `wasmtime::ValType`.
  [#6138](https://github.com/bytecodealliance/wasmtime/pull/6138)

* Make `StoreContextMut` accessible in the epoch deadline callback.
  [#6075](https://github.com/bytecodealliance/wasmtime/pull/6075)

* Take SIGFPE signals for divide traps on `x86_64`.
  [#6026](https://github.com/bytecodealliance/wasmtime/pull/6026)

* Use more specialized AVX instructions in the `x86_64` backend.
  [#5924](https://github.com/bytecodealliance/wasmtime/pull/5924)
  [#5930](https://github.com/bytecodealliance/wasmtime/pull/5930)
  [#5931](https://github.com/bytecodealliance/wasmtime/pull/5931)
  [#5982](https://github.com/bytecodealliance/wasmtime/pull/5982)
  [#5986](https://github.com/bytecodealliance/wasmtime/pull/5986)
  [#5999](https://github.com/bytecodealliance/wasmtime/pull/5999)
  [#6023](https://github.com/bytecodealliance/wasmtime/pull/6023)
  [#6025](https://github.com/bytecodealliance/wasmtime/pull/6025)
  [#6060](https://github.com/bytecodealliance/wasmtime/pull/6060)
  [#6086](https://github.com/bytecodealliance/wasmtime/pull/6086)
  [#6092](https://github.com/bytecodealliance/wasmtime/pull/6092)

* Generate more cache-friendly code for traps.
  [#6011](https://github.com/bytecodealliance/wasmtime/pull/6011)

### Fixed

* Fixed suboptimal code generation in the `aarch64` backend.
  [#5976](https://github.com/bytecodealliance/wasmtime/pull/5976)
  [#5977](https://github.com/bytecodealliance/wasmtime/pull/5977)
  [#5987](https://github.com/bytecodealliance/wasmtime/pull/5987)
  [#5997](https://github.com/bytecodealliance/wasmtime/pull/5997)
  [#6078](https://github.com/bytecodealliance/wasmtime/pull/6078)

* Fixed suboptimal code generation in the `riscv64` backend.
  [#5854](https://github.com/bytecodealliance/wasmtime/pull/5854)
  [#5857](https://github.com/bytecodealliance/wasmtime/pull/5857)
  [#5919](https://github.com/bytecodealliance/wasmtime/pull/5919)
  [#5951](https://github.com/bytecodealliance/wasmtime/pull/5951)
  [#5964](https://github.com/bytecodealliance/wasmtime/pull/5964)
  [#6087](https://github.com/bytecodealliance/wasmtime/pull/6087)


--------------------------------------------------------------------------------

## 7.0.1

Released 2023-04-27.

### Fixed

* Undefined Behavior in Rust runtime functions
  [GHSA-ch89-5g45-qwc7](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-ch89-5g45-qwc7)

--------------------------------------------------------------------------------

## 7.0.0

Released 2023-03-20

### Added

* An initial implementation of the wasi-threads proposal has been implemented
  and landed in the Wasmtime CLI. This is available behind a
  `--wasi-modules experimental-wasi-threads` flag.
  [#5484](https://github.com/bytecodealliance/wasmtime/pull/5484)

* Support for WASI sockets has been added to the C API.
  [#5624](https://github.com/bytecodealliance/wasmtime/pull/5624)

* Support for limiting `Store`-based resource usage, such as memory, tables,
  etc, has been added to the C API.
  [#5761](https://github.com/bytecodealliance/wasmtime/pull/5761)

* A top level alias of `anyhow::Result` as `wasmtime::Result` has been added to
  avoid the need to explicitly depend on `anyhow`.
  [#5853](https://github.com/bytecodealliance/wasmtime/pull/5853)

* Initial support for the WebAssembly core dump format has been added to the CLI
  with a `--coredump-on-trap` flag.
  [#5868](https://github.com/bytecodealliance/wasmtime/pull/5868)

### Changed

* The `S` type parameter on component-related methods has been removed.
  [#5722](https://github.com/bytecodealliance/wasmtime/pull/5722)

* Selection of a `world` to bindgen has been updated to select any `default
  world` in a WIT package if there is only one.
  [#5779](https://github.com/bytecodealliance/wasmtime/pull/5779)

* WASI preopened file descriptors can now be closed.
  [#5828](https://github.com/bytecodealliance/wasmtime/pull/5828)

* The host traits generated by the `bindgen!` macro are now always named `Host`,
  but are still scoped to each individual module.
  [#5890](https://github.com/bytecodealliance/wasmtime/pull/5890)

### Fixed

* Components which have `type` imports are now supported better and error/panic
  in fewer cases.
  [#5777](https://github.com/bytecodealliance/wasmtime/pull/5777)

* Types referred to by `wasmtime::component::Val` are now reexported under
  `wasmtime::component`.
  [#5790](https://github.com/bytecodealliance/wasmtime/pull/5790)

* A panic due to a race between `memory.atomic.{wait32,wait64,notify}`
  instructions has been fixed.
  [#5871](https://github.com/bytecodealliance/wasmtime/pull/5871)

* Guest-controlled out-of-bounds read/write on x86\_64
  [GHSA-ff4p-7xrq-q5r8](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-ff4p-7xrq-q5r8)

*  Miscompilation of `i8x16.select` with the same inputs on x86\_64
  [GHSA-xm67-587q-r2vw](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-xm67-587q-r2vw)

--------------------------------------------------------------------------------

## 6.0.2

Released 2023-04-27.

### Fixed

* Undefined Behavior in Rust runtime functions
  [GHSA-ch89-5g45-qwc7](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-ch89-5g45-qwc7)

--------------------------------------------------------------------------------

## 6.0.1

Released 2023-03-08.

### Fixed

* Guest-controlled out-of-bounds read/write on x86\_64
  [GHSA-ff4p-7xrq-q5r8](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-ff4p-7xrq-q5r8)

*  Miscompilation of `i8x16.select` with the same inputs on x86\_64
  [GHSA-xm67-587q-r2vw](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-xm67-587q-r2vw)

--------------------------------------------------------------------------------

## 6.0.0

Released 2023-02-20

### Added

* Wasmtime's built-in cache can now be disabled after being enabled previously.
  [#5542](https://github.com/bytecodealliance/wasmtime/pull/5542)

* Older x86\_64 CPUs, without SSE4.1 for example, are now supported when the
  wasm SIMD proposal is disabled.
  [#5567](https://github.com/bytecodealliance/wasmtime/pull/5567)

* The Wasmtime C API now has `WASMTIME_VERSION_*` macros defined in its header
  files.
  [#5651](https://github.com/bytecodealliance/wasmtime/pull/5651)

* The `wasmtime` CLI executable as part of Wasmtime's precompiled release
  artifacts now has the `all-arch` feature enabled.
  [#5657](https://github.com/bytecodealliance/wasmtime/pull/5657)

### Changed

* Equality of `wasmtime::component::Val::Float{32,64}` now considers NaNs as
  equal for assistance when fuzzing.
  [#5535](https://github.com/bytecodealliance/wasmtime/pull/5535)

* WIT syntax supported by `wasmtime::component::bindgen!` has been updated in
  addition to the generated code being updated.
  [#5565](https://github.com/bytecodealliance/wasmtime/pull/5565)
  [#5692](https://github.com/bytecodealliance/wasmtime/pull/5692)
  [#5694](https://github.com/bytecodealliance/wasmtime/pull/5694)

* Cranelift's egraph-based optimization framework is now enabled by default.
  [#5587](https://github.com/bytecodealliance/wasmtime/pull/5587)

* The old `PoolingAllocationStrategy` type has been removed in favor of a more
  flexible configuration via a new option
  `PoolingAllocationConfig::max_unused_warm_slots` which is more flexible and
  subsumes the previous use cases for each strategy.
  [#5661](https://github.com/bytecodealliance/wasmtime/pull/5661)

* Creation of `InstancePre` through `Linker::instantiate_pre` no longer requires
  a `Store` to be provided. Instead a `Store`-related argument is now required
  on `Linker::define`-style APIs instead.
  [#5683](https://github.com/bytecodealliance/wasmtime/pull/5683)

### Fixed

* Compilation for FreeBSD on x86\_64 and AArch64 has been fixed.
  [#5606](https://github.com/bytecodealliance/wasmtime/pull/5606)

--------------------------------------------------------------------------------

## 5.0.1

Released 2023-03-08.

### Fixed

* Guest-controlled out-of-bounds read/write on x86\_64
  [GHSA-ff4p-7xrq-q5r8](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-ff4p-7xrq-q5r8)

*  Miscompilation of `i8x16.select` with the same inputs on x86\_64
  [GHSA-xm67-587q-r2vw](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-xm67-587q-r2vw)

--------------------------------------------------------------------------------

## 5.0.0

Released 2023-01-20

### Added

* A `wasmtime::component::bingen!` macro has been added for generating bindings
  from `*.wit` files. Note that WIT is still heavily in development so this is
  more of a preview of what will be as opposed to a finished feature.
  [#5317](https://github.com/bytecodealliance/wasmtime/pull/5317)
  [#5397](https://github.com/bytecodealliance/wasmtime/pull/5397)

* The `wasmtime settings` CLI command now has a `--json` option for
  machine-readable output.
  [#5411](https://github.com/bytecodealliance/wasmtime/pull/5411)

* Wiggle-generated bindings can now generate the trait for either `&mut self` or
  `&self`.
  [#5428](https://github.com/bytecodealliance/wasmtime/pull/5428)

* The `wiggle` crate has more convenience APIs for working with guest data
  that resides in shared memory.
  [#5471](https://github.com/bytecodealliance/wasmtime/pull/5471)
  [#5475](https://github.com/bytecodealliance/wasmtime/pull/5475)

### Changed

* Cranelift's egraph support has been rewritten and updated. This functionality
  is still gated behind a flag and may become the default in the next release.
  [#5382](https://github.com/bytecodealliance/wasmtime/pull/5382)

* The implementation of codegen for WebAssembly linear memory has changed
  significantly internally in Cranelift, moving more responsibility to the
  Wasmtime embedding rather than Cranelift itself. This should have no
  user-visible change, however.
  [#5386](https://github.com/bytecodealliance/wasmtime/pull/5386)

* The `Val::Float32` and `Val::Float64` variants for components now store `f32`
  and `f64` instead of the bit representation.
  [#5510](https://github.com/bytecodealliance/wasmtime/pull/5510)

### Fixed

* Handling of DWARF debugging information in components with multiple modules
  has been fixed to ensure the right info is used for each module.
  [#5358](https://github.com/bytecodealliance/wasmtime/pull/5358)

--------------------------------------------------------------------------------

## 4.0.1

Released 2023-03-08.

### Fixed

* Guest-controlled out-of-bounds read/write on x86\_64
  [GHSA-ff4p-7xrq-q5r8](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-ff4p-7xrq-q5r8)

*  Miscompilation of `i8x16.select` with the same inputs on x86\_64
  [GHSA-xm67-587q-r2vw](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-xm67-587q-r2vw)

--------------------------------------------------------------------------------

## 4.0.0

Released 2022-12-20

### Added

* Dynamic memories are now supported with the pooling instance allocator which
  can possibly reduce the number of page faults throughout execution at the cost
  of slower to run code. Page faults are primarily reduced by avoiding
  releasing memory back to the system, relying on bounds checks to keep the
  memory inaccessible.
  [#5208](https://github.com/bytecodealliance/wasmtime/pull/5208)

* The `wiggle` generator now supports function-level control over `tracing`
  calls.
  [#5194](https://github.com/bytecodealliance/wasmtime/pull/5194)

* Support has been added to `wiggle` to be compatible with shared memories.
  [#5225](https://github.com/bytecodealliance/wasmtime/pull/5225)
  [#5229](https://github.com/bytecodealliance/wasmtime/pull/5229)
  [#5264](https://github.com/bytecodealliance/wasmtime/pull/5264)
  [#5268](https://github.com/bytecodealliance/wasmtime/pull/5268)
  [#5054](https://github.com/bytecodealliance/wasmtime/pull/5054)

* The `wiggle` generator now supports a "trappable error" configuration to
  improve error conversions to guest errors and ensure that no host errors are
  forgotten or accidentally become traps. The `wasi-common` crate has been
  updated to use this.
  [#5276](https://github.com/bytecodealliance/wasmtime/pull/5276)
  [#5279](https://github.com/bytecodealliance/wasmtime/pull/5279)

* The `memory.atomic.{notify,wait32,wait64}` instructions are now all
  implemented in Wasmtime.
  [#5255](https://github.com/bytecodealliance/wasmtime/pull/5255)
  [#5311](https://github.com/bytecodealliance/wasmtime/pull/5311)

* A `wasm_config_parallel_compilation_set` configuration function has been added
  to the C API.
  [#5298](https://github.com/bytecodealliance/wasmtime/pull/5298)

* The `wasmtime` CLI can have its input module piped into it from stdin now.
  [#5342](https://github.com/bytecodealliance/wasmtime/pull/5342)

* `WasmBacktrace::{capture,force_capture}` methods have been added to
  programmatically capture a backtrace outside of a trapping context.
  [#5341](https://github.com/bytecodealliance/wasmtime/pull/5341)

### Changed

* The `S` type parameter on `Func::typed` and `Instance::get_typed_func` has
  been removed and no longer needs to be specified.
  [#5275](https://github.com/bytecodealliance/wasmtime/pull/5275)

* The `SharedMemory::data` method now returns `&[UnsafeCell<u8>]` instead of the
  prior raw slice return.
  [#5240](https://github.com/bytecodealliance/wasmtime/pull/5240)

* Creation of a `WasiCtx` will no longer unconditionally acquire randomness from
  the OS, instead using the `rand::thread_rng()` function in Rust which is only
  periodically reseeded with randomness from the OS.
  [#5244](https://github.com/bytecodealliance/wasmtime/pull/5244)

* Codegen of dynamically-bounds-checked wasm memory accesses has been improved.
  [#5190](https://github.com/bytecodealliance/wasmtime/pull/5190)

* Wasmtime will now emit inline stack probes in generated functions for x86\_64,
  aarch64, and riscv64 architectures. This guarantees a process abort if an
  engine was misconfigured to give wasm too much stack instead of optionally
  allowing wasm to skip the guard page.
  [#5350](https://github.com/bytecodealliance/wasmtime/pull/5350)
  [#5353](https://github.com/bytecodealliance/wasmtime/pull/5353)

### Fixed

* Dropping a `Module` will now release kernel resources in-use by the pooling
  allocator when enabled instead of waiting for a new instance to be
  re-instantiated into prior slots.
  [#5321](https://github.com/bytecodealliance/wasmtime/pull/5321)

--------------------------------------------------------------------------------

## 3.0.1

Released 2022-12-01.

### Fixed

* The instruction cache is now flushed for AArch64 Android.
  [#5331](https://github.com/bytecodealliance/wasmtime/pull/5331)

* Building for FreeBSD and Android has been fixed.
  [#5323](https://github.com/bytecodealliance/wasmtime/pull/5323)

--------------------------------------------------------------------------------

## 3.0.0

Released 2022-11-21

### Added

* New `WasiCtx::{push_file, push_dir}` methods exist for embedders to add their
  own objects.
  [#5027](https://github.com/bytecodealliance/wasmtime/pull/5027)

* Wasmtime's `component-model` support now supports `async` host functions and
  embedding in the same manner as core wasm.
  [#5055](https://github.com/bytecodealliance/wasmtime/pull/5055)

* The `wasmtime` CLI executable now supports a `--max-wasm-stack` flag.
  [#5156](https://github.com/bytecodealliance/wasmtime/pull/5156)

* AOT compilation support has been implemented for components (aka the
  `component-model` feature of the Wasmtime crate).
  [#5160](https://github.com/bytecodealliance/wasmtime/pull/5160)

* A new `wasi_config_set_stdin_bytes` function is available in the C API to set
  the stdin of a WASI-using module from an in-memory slice.
  [#5179](https://github.com/bytecodealliance/wasmtime/pull/5179)

* When using the pooling allocator there are now options to reset memory with
  `memset` instead of `madvisev` on Linux to keep pages resident in memory to
  reduce page faults when reusing linear memory slots.
  [#5207](https://github.com/bytecodealliance/wasmtime/pull/5207)

### Changed

* Consuming 0 fuel with 0 fuel left is now considered to succeed. Additionally a
  store may not consume its last unit of fuel.
  [#5013](https://github.com/bytecodealliance/wasmtime/pull/5013)

* A number of variants in the `wasi_common::ErrorKind` enum have been removed.
  [#5015](https://github.com/bytecodealliance/wasmtime/pull/5015)

* Methods on `WasiDir` now error-by-default instead of requiring a definition by
  default.
  [#5019](https://github.com/bytecodealliance/wasmtime/pull/5019)

* Bindings generated by the `wiggle` crate now always depend on the `wasmtime`
  crate meaning crates like `wasi-common` no longer compile for platforms such
  as `wasm32-unknown-emscripten`.
  [#5137](https://github.com/bytecodealliance/wasmtime/pull/5137)

* Error handling in the `wasmtime` crate's API has been changed to primarily
  work with `anyhow::Error` for custom errors. The `Trap` type has been replaced
  with a simple `enum Trap { ... }` and backtrace information is now stored as a
  `WasmBacktrace` type inserted as context into an `anyhow::Error`.
  Host-functions are expected to return `anyhow::Result<T>` instead of the prior
  `Trap` error return from before. Additionally the old `Trap::i32_exit`
  constructor is now a concrete `wasi_commont::I32Exit` type which can be tested
  for with a `downcast_ref` on the error returned from Wasmtime.
  [#5149](https://github.com/bytecodealliance/wasmtime/pull/5149)

* Configuration of the pooling allocator is now done through a builder-style
  `PoolingAllocationConfig` API instead of the prior enum-variant API.
  [#5205](https://github.com/bytecodealliance/wasmtime/pull/5205)

### Fixed

* The instruction cache is now properly flushed for AArch64 on Windows.
  [#4997](https://github.com/bytecodealliance/wasmtime/pull/4997)

* Backtrace capturing with many sequences of wasm->host calls on the stack no
  longer exhibit quadratic capturing behavior.
  [#5049](https://github.com/bytecodealliance/wasmtime/pull/5049)

--------------------------------------------------------------------------------

## 2.0.2

Released 2022-11-10.

### Fixed

* [CVE-2022-39392] - modules may perform out-of-bounds reads/writes when the
  pooling allocator was configured with `memory_pages: 0`.

* [CVE-2022-39393] - data can be leaked between instances when using the pooling
  allocator.

* [CVE-2022-39394] - An incorrect Rust signature for the C API
  `wasmtime_trap_code` function could lead to an out-of-bounds write of three
  zero bytes.

[CVE-2022-39392]: https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-44mr-8vmm-wjhg
[CVE-2022-39393]: https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-wh6w-3828-g9qf
[CVE-2022-39394]: https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-h84q-m8rr-3v9q

--------------------------------------------------------------------------------

## 2.0.1

Released 2022-10-27.

### Fixed

* A compilation error when building only the `wasmtime` crate on Windows with
  only the default features enabled has been fixed.
  [#5134](https://github.com/bytecodealliance/wasmtime/pull/5134)

### Changed

* The `rayon` dependency added to `cranelift-isle` in 2.0.0 has been removed to
  improve the compile time of the `cranelift-codegen` crate.
  [#5101](https://github.com/bytecodealliance/wasmtime/pull/5101)

--------------------------------------------------------------------------------

## 2.0.0

Released 2022-10-20

### Added

* Cranelift has gained support for forward-edge CFI on the AArch64 backend.
  [#3693](https://github.com/bytecodealliance/wasmtime/pull/3693)

* A `--disable-parallel-compilation` CLI flag is now implemented for `wasmtime`.
  [#4911](https://github.com/bytecodealliance/wasmtime/pull/4911)

* [Tier 3] support has been added for for RISC-V 64 with a new backend in
  Cranelift for this architecture.
  [#4271](https://github.com/bytecodealliance/wasmtime/pull/4271)

* Basic [tier 3] support for Windows ARM64 has been added but features such as
  traps don't work at this time.
  [#4990](https://github.com/bytecodealliance/wasmtime/pull/4990)

### Changed

* The implementation of the `random_get` function in `wasi-common` is now faster
  by using a userspace CSPRNG rather than the OS for randomness.
  [#4917](https://github.com/bytecodealliance/wasmtime/pull/4917)

* The AArch64 backend has completed its transition to ISLE.
  [#4851](https://github.com/bytecodealliance/wasmtime/pull/4851)
  [#4866](https://github.com/bytecodealliance/wasmtime/pull/4866)
  [#4898](https://github.com/bytecodealliance/wasmtime/pull/4898)
  [#4884](https://github.com/bytecodealliance/wasmtime/pull/4884)
  [#4820](https://github.com/bytecodealliance/wasmtime/pull/4820)
  [#4913](https://github.com/bytecodealliance/wasmtime/pull/4913)
  [#4942](https://github.com/bytecodealliance/wasmtime/pull/4942)
  [#4943](https://github.com/bytecodealliance/wasmtime/pull/4943)

* The size of the `sigaltstack` allocated per-thread for signal handling has
  been increased from 16k to 64k.
  [#4964](https://github.com/bytecodealliance/wasmtime/pull/4964)


[Tier 3]: https://docs.wasmtime.dev/stability-tiers.html

--------------------------------------------------------------------------------

## 1.0.2

Released 2022-11-10.

### Fixed

* [CVE-2022-39392] - modules may perform out-of-bounds reads/writes when the
  pooling allocator was configured with `memory_pages: 0`.

* [CVE-2022-39393] - data can be leaked between instances when using the pooling
  allocator.

* [CVE-2022-39394] - An incorrect Rust signature for the C API
  `wasmtime_trap_code` function could lead to an out-of-bounds write of three
  zero bytes.

--------------------------------------------------------------------------------

## 1.0.1

Released 2022-09-26

This is a patch release that incorporates a fix for a miscompilation of an
atomic-CAS operator on aarch64. The instruction is not usable from Wasmtime
with default settings, but may be used if the Wasm atomics extension is
enabled. The bug may also be reachable via other uses of Cranelift. Thanks to
@bjorn3 for reporting and debugging this issue!

### Fixed

* Fixed a miscompilation of `atomic_cas` on aarch64. The output register was
  swapped with a temporary register in the register-allocator constraints.
  [#4959](https://github.com/bytecodealliance/wasmtime/pull/4959)
  [#4960](https://github.com/bytecodealliance/wasmtime/pull/4960)

--------------------------------------------------------------------------------

## 1.0.0

Released 2022-09-20

This release marks the official 1.0 release of Wasmtime and represents the
culmination of the work amongst over 300 contributors. Wasmtime has been
battle-tested in production through multiple embeddings for quite some time now
and we're confident in releasing a 1.0 version to signify the stability and
quality of the Wasmtime engine.

More information about Wasmtime's 1.0 release is on the [Bytecode Alliance's
blog][ba-blog] with separate posts on [Wasmtime's performance
features][ba-perf], [Wasmtime's security story][ba-security], and [the 1.0
release announcement][ba-1.0].

As a reminder the 2.0 release of Wasmtime is scheduled for one month from now on
October 20th. For more information see the [RFC on Wasmtime's 1.0
release][rfc-1.0].

[ba-blog]: https://bytecodealliance.org/articles/
[ba-perf]: https://bytecodealliance.org/articles/wasmtime-10-performance
[ba-security]: https://bytecodealliance.org/articles/security-and-correctness-in-wasmtime
[ba-1.0]: https://bytecodealliance.org/articles/wasmtime-1-0-fast-safe-and-now-production-ready.md
[rfc-1.0]: https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-one-dot-oh.md

### Added

* An incremental compilation cache for Cranelift has been added which can be
  enabled with `Config::enable_incremental_compilation`, and this option is
  disabled by default for now. The incremental compilation cache has been
  measured to improve compile times for cold uncached modules as well due to
  some wasm modules having similar-enough functions internally.
  [#4551](https://github.com/bytecodealliance/wasmtime/pull/4551)

* Source tarballs are now available as part of Wasmtime's release artifacts.
  [#4294](https://github.com/bytecodealliance/wasmtime/pull/4294)

* WASI APIs that specify the REALTIME clock are now supported.
  [#4777](https://github.com/bytecodealliance/wasmtime/pull/4777)

* WASI's socket functions are now fully implemented.
  [#4776](https://github.com/bytecodealliance/wasmtime/pull/4776)

* The native call stack for async-executed wasm functions are no longer
  automatically reset to zero after the stack is returned to the pool when using
  the pooling allocator. A `Config::async_stack_zeroing` option has been added
  to restore the old behavior of zero-on-return-to-pool.
  [#4813](https://github.com/bytecodealliance/wasmtime/pull/4813)

* Inline stack probing has been implemented for the Cranelift x64 backend.
  [#4747](https://github.com/bytecodealliance/wasmtime/pull/4747)

### Changed

* Generating of native unwind information has moved from a
  `Config::wasm_backtrace` option to a new `Config::native_unwind_info` option
  and is enabled by default.
  [#4643](https://github.com/bytecodealliance/wasmtime/pull/4643)

* The `memory-init-cow` feature is now enabled by default in the C API.
  [#4690](https://github.com/bytecodealliance/wasmtime/pull/4690)

* Back-edge CFI is now enabled by default on AArch64 macOS.
  [#4720](https://github.com/bytecodealliance/wasmtime/pull/4720)

* WASI calls will no longer return NOTCAPABLE in preparation for the removal of
  the rights system from WASI.
  [#4666](https://github.com/bytecodealliance/wasmtime/pull/4666)

### Internal

This section of the release notes shouldn't affect external users since no
public-facing APIs are affected, but serves as a place to document larger
changes internally within Wasmtime.

* Differential fuzzing has been refactored and improved into one fuzzing target
  which can execute against any of Wasmtime itself (configured differently),
  wasmi, V8, or the spec interpreter. Fuzzing now executes each exported
  function with fuzz-generated inputs and the contents of all of memory and each
  exported global is compared after each execution. Additionally more
  interesting shapes of modules are also possible to generate.
  [#4515](https://github.com/bytecodealliance/wasmtime/pull/4515)
  [#4735](https://github.com/bytecodealliance/wasmtime/pull/4735)
  [#4737](https://github.com/bytecodealliance/wasmtime/pull/4737)
  [#4739](https://github.com/bytecodealliance/wasmtime/pull/4739)
  [#4774](https://github.com/bytecodealliance/wasmtime/pull/4774)
  [#4773](https://github.com/bytecodealliance/wasmtime/pull/4773)
  [#4845](https://github.com/bytecodealliance/wasmtime/pull/4845)
  [#4672](https://github.com/bytecodealliance/wasmtime/pull/4672)
  [#4674](https://github.com/bytecodealliance/wasmtime/pull/4674)

* The x64 backend for Cranelift has been fully migrated to ISLE.
  [#4619](https://github.com/bytecodealliance/wasmtime/pull/4619)
  [#4625](https://github.com/bytecodealliance/wasmtime/pull/4625)
  [#4645](https://github.com/bytecodealliance/wasmtime/pull/4645)
  [#4650](https://github.com/bytecodealliance/wasmtime/pull/4650)
  [#4684](https://github.com/bytecodealliance/wasmtime/pull/4684)
  [#4704](https://github.com/bytecodealliance/wasmtime/pull/4704)
  [#4718](https://github.com/bytecodealliance/wasmtime/pull/4718)
  [#4726](https://github.com/bytecodealliance/wasmtime/pull/4726)
  [#4722](https://github.com/bytecodealliance/wasmtime/pull/4722)
  [#4729](https://github.com/bytecodealliance/wasmtime/pull/4729)
  [#4730](https://github.com/bytecodealliance/wasmtime/pull/4730)
  [#4741](https://github.com/bytecodealliance/wasmtime/pull/4741)
  [#4763](https://github.com/bytecodealliance/wasmtime/pull/4763)
  [#4772](https://github.com/bytecodealliance/wasmtime/pull/4772)
  [#4780](https://github.com/bytecodealliance/wasmtime/pull/4780)
  [#4787](https://github.com/bytecodealliance/wasmtime/pull/4787)
  [#4793](https://github.com/bytecodealliance/wasmtime/pull/4793)
  [#4809](https://github.com/bytecodealliance/wasmtime/pull/4809)

* The AArch64 backend for Cranelift has seen significant progress in being
  ported to ISLE.
  [#4608](https://github.com/bytecodealliance/wasmtime/pull/4608)
  [#4639](https://github.com/bytecodealliance/wasmtime/pull/4639)
  [#4634](https://github.com/bytecodealliance/wasmtime/pull/4634)
  [#4748](https://github.com/bytecodealliance/wasmtime/pull/4748)
  [#4750](https://github.com/bytecodealliance/wasmtime/pull/4750)
  [#4751](https://github.com/bytecodealliance/wasmtime/pull/4751)
  [#4753](https://github.com/bytecodealliance/wasmtime/pull/4753)
  [#4788](https://github.com/bytecodealliance/wasmtime/pull/4788)
  [#4796](https://github.com/bytecodealliance/wasmtime/pull/4796)
  [#4785](https://github.com/bytecodealliance/wasmtime/pull/4785)
  [#4819](https://github.com/bytecodealliance/wasmtime/pull/4819)
  [#4821](https://github.com/bytecodealliance/wasmtime/pull/4821)
  [#4832](https://github.com/bytecodealliance/wasmtime/pull/4832)

* The s390x backend has seen improvements and additions to fully support the
  Cranelift backend for rustc.
  [#4682](https://github.com/bytecodealliance/wasmtime/pull/4682)
  [#4702](https://github.com/bytecodealliance/wasmtime/pull/4702)
  [#4616](https://github.com/bytecodealliance/wasmtime/pull/4616)
  [#4680](https://github.com/bytecodealliance/wasmtime/pull/4680)

* Significant improvements have been made to Cranelift-based fuzzing with more
  supported features and more instructions being fuzzed.
  [#4589](https://github.com/bytecodealliance/wasmtime/pull/4589)
  [#4591](https://github.com/bytecodealliance/wasmtime/pull/4591)
  [#4665](https://github.com/bytecodealliance/wasmtime/pull/4665)
  [#4670](https://github.com/bytecodealliance/wasmtime/pull/4670)
  [#4590](https://github.com/bytecodealliance/wasmtime/pull/4590)
  [#4375](https://github.com/bytecodealliance/wasmtime/pull/4375)
  [#4519](https://github.com/bytecodealliance/wasmtime/pull/4519)
  [#4696](https://github.com/bytecodealliance/wasmtime/pull/4696)
  [#4700](https://github.com/bytecodealliance/wasmtime/pull/4700)
  [#4703](https://github.com/bytecodealliance/wasmtime/pull/4703)
  [#4602](https://github.com/bytecodealliance/wasmtime/pull/4602)
  [#4713](https://github.com/bytecodealliance/wasmtime/pull/4713)
  [#4738](https://github.com/bytecodealliance/wasmtime/pull/4738)
  [#4667](https://github.com/bytecodealliance/wasmtime/pull/4667)
  [#4782](https://github.com/bytecodealliance/wasmtime/pull/4782)
  [#4783](https://github.com/bytecodealliance/wasmtime/pull/4783)
  [#4800](https://github.com/bytecodealliance/wasmtime/pull/4800)

* Optimization work on cranelift has continued across various dimensions for
  some modest compile-time improvements.
  [#4621](https://github.com/bytecodealliance/wasmtime/pull/4621)
  [#4701](https://github.com/bytecodealliance/wasmtime/pull/4701)
  [#4697](https://github.com/bytecodealliance/wasmtime/pull/4697)
  [#4711](https://github.com/bytecodealliance/wasmtime/pull/4711)
  [#4710](https://github.com/bytecodealliance/wasmtime/pull/4710)
  [#4829](https://github.com/bytecodealliance/wasmtime/pull/4829)

--------------------------------------------------------------------------------

## 0.40.0

Released 2022-08-20

This was a relatively quiet release in terms of user-facing features where most
of the work was around the internals of Wasmtime and Cranelift. Improvements
internally have been made along the lines of:

* Many more instructions are now implemented with ISLE instead of handwritten
  lowerings.
* Many improvements to the cranelift-based fuzzing.
* Many platform improvements for s390x including full SIMD support, running
  `rustc_codegen_cranelift` with features like `i128`, supporting more
  ABIs, etc.
* Much more of the component model has been implemented and is now fuzzed.

Finally this release is currently scheduled to be the last `0.*` release of
Wasmtime. The upcoming release of Wasmtime on September 20 is planned to be
Wasmtime's 1.0 release. More information about what 1.0 means for Wasmtime is
available in the [1.0 RFC]

[1.0 RFC]: https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-one-dot-oh.md

### Added

* Stack walking has been reimplemented with frame pointers rather than with
  native unwind information. This means that backtraces are feasible to capture
  in performance-critical environments and in general stack walking is much
  faster than before.
  [#4431](https://github.com/bytecodealliance/wasmtime/pull/4431)

* The WebAssembly `simd` proposal is now fully implemented for the s390x
  backend.
  [#4427](https://github.com/bytecodealliance/wasmtime/pull/4427)

* Support for AArch64 has been added in the experimental native debuginfo
  support that Wasmtime has.
  [#4468](https://github.com/bytecodealliance/wasmtime/pull/4468)

* Support building the C API of Wasmtime with CMake has been added.
  [#4369](https://github.com/bytecodealliance/wasmtime/pull/4369)

* Clarification was added to Wasmtime's documentation about "tiers of support"
  for various features.
  [#4479](https://github.com/bytecodealliance/wasmtime/pull/4479)

### Fixed

* Support for `filestat_get` has been improved for stdio streams in WASI.
  [#4531](https://github.com/bytecodealliance/wasmtime/pull/4531)

* Enabling the `vtune` feature no longer breaks builds on AArch64.
  [#4533](https://github.com/bytecodealliance/wasmtime/pull/4533)

--------------------------------------------------------------------------------

## 0.39.1

Released 2022-07-20.

### Fixed

* An s390x-specific codegen bug in addition to a mistake introduced in the fix
  of CVE-2022-31146 were fixed.
  [#4490](https://github.com/bytecodealliance/wasmtime/pull/4490)

--------------------------------------------------------------------------------

## 0.39.0

Released 2022-07-20

### Added

* Initial support for shared memories and the `threads` WebAssembly proposal
  has been added. Note that this feature is still experimental and not ready
  for production use yet.
  [#4187](https://github.com/bytecodealliance/wasmtime/pull/4187)

* A new `Linker::define_unknown_imports_as_traps` method and
  `--trap-unknown-imports` CLI flag have been added to conveniently support
  running modules with imports that aren't dynamically called at runtime.
  [#4312](https://github.com/bytecodealliance/wasmtime/pull/4312)

* The VTune profiling strategy can now be selected through the C API.
  [#4316](https://github.com/bytecodealliance/wasmtime/pull/4316)

### Changed

* Some methods on the `Config` structure now return `&mut Self` instead of
  `Result<&mut Self>` since the validation is deferred until `Engine::new`:
  `profiler`, `cranelift_flag_enable`, `cranelift_flag_set`, `max_wasm_stack`,
  `async_stack_size`, and `strategy`.
  [#4252](https://github.com/bytecodealliance/wasmtime/pull/4252)
  [#4262](https://github.com/bytecodealliance/wasmtime/pull/4262)

* Parallel compilation of WebAssembly modules is now enabled in the C API by
  default.
  [#4270](https://github.com/bytecodealliance/wasmtime/pull/4270)

* Implicit Cargo features of the `wasmtime` introduced through `optional`
  dependencies may have been removed since namespaced features are now used.
  It's recommended to only used the set of named `[features]` for Wasmtime.
  [#4293](https://github.com/bytecodealliance/wasmtime/pull/4293)

* Register allocation has fixed a few issues related to excessive memory usage
  at compile time.
  [#4324](https://github.com/bytecodealliance/wasmtime/pull/4324)

### Fixed

* A refactor of `Config` was made to fix an issue that the order of calls to `Config`
  matters now, which may lead to unexpected behavior.
  [#4252](https://github.com/bytecodealliance/wasmtime/pull/4252)
  [#4262](https://github.com/bytecodealliance/wasmtime/pull/4262)

* Wasmtime has been fixed to work on SSE2-only x86\_64 platforms when the
  `simd` feature is disabled in `Config`.
  [#4231](https://github.com/bytecodealliance/wasmtime/pull/4231)

* Generation of platform-specific unwinding information is disabled if
  `wasm_backtrace` and `wasm_reference_types` are both disabled.
  [#4351](https://github.com/bytecodealliance/wasmtime/pull/4351)

--------------------------------------------------------------------------------

## 0.38.3

Released 2022-07-20.

### Fixed.

* An s390x-specific codegen bug in addition to a mistake introduced in the fix
  of CVE-2022-31146 were fixed.
  [#4491](https://github.com/bytecodealliance/wasmtime/pull/4491)

--------------------------------------------------------------------------------

## 0.38.2

Released 2022-07-20.

### Fixed.

* A miscompilation when handling constant divisors on AArch64 has been fixed.
  [CVE-2022-31169](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-7f6x-jwh5-m9r4)

* A use-after-free possible with accidentally missing stack maps has been fixed.
  [CVE-2022-31146](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-5fhj-g3p3-pq9g)

--------------------------------------------------------------------------------

## 0.38.1

Released 2022-06-27.

### Fixed.

* A register allocator bug was fixed that could affect direct users of
  Cranelift who use struct-return (`sret`) arguments. The bug had to do with
  the handling of physical register constraints in the function prologue. No
  impact should be possible for users of Cranelift via the Wasm frontend,
  including Wasmtime.
  [regalloc2#60](https://github.com/bytecodealliance/regalloc2/pull/60)
  [#4333](https://github.com/bytecodealliance/wasmtime/pull/4333)

* Lowering bugs for the `i8x16.swizzle` and `select`-with-`v128`-inputs
  instructions were fixed for the x86\_64 code generator. Note that aarch64 and
  s390x are unaffected.
  [#4334](https://github.com/bytecodealliance/wasmtime/pull/4334)

* A bug in the 8-bit lowering of integer division on x86-64 was fixed in
  Cranelift that could cause a register allocator panic due to an undefined
  value in a register. (The divide instruction does not take a register `rdx`
  as a source when 8 bits but the metadata incorrectly claimed it did.) No
  impact on Wasm/Wasmtime users, and impact on direct Cranelift embedders
  limited to compilation panics.
  [#4332](https://github.com/bytecodealliance/wasmtime/pull/4332)

--------------------------------------------------------------------------------

## 0.38.0

Released 2022-06-21

### Added

* Enabling or disabling NaN canonicalization in generated code is now exposed
  through the C API.
  [#4154](https://github.com/bytecodealliance/wasmtime/pull/4154)

* A user-defined callback can now be invoked when an epoch interruption happens
  via the `Store::epoch_deadline_callback` API.
  [#4152](https://github.com/bytecodealliance/wasmtime/pull/4152)

* Basic alias analysis with redundant-load elimintation and store-to-load
  forwarding optimizations has been added to Cranelift.
  [#4163](https://github.com/bytecodealliance/wasmtime/pull/4163)

### Changed

* Traps originating from epoch-based interruption are now exposed as
  `TrapCode::Interrupt`.
  [#4105](https://github.com/bytecodealliance/wasmtime/pull/4105)

* Binary builds for AArch64 now require glibc 2.17 and for s390x require glibc
  2.16. Previously glibc 2.28 was required.
  [#4171](https://github.com/bytecodealliance/wasmtime/pull/4171)

* The `wasmtime::ValRaw` now has all of its fields listed as private and instead
  constructors/accessors are provided for getting at the internal data.
  [#4186](https://github.com/bytecodealliance/wasmtime/pull/4186)

* The `wasm-backtrace` Cargo feature has been removed in favor of a
  `Config::wasm_backtrace` runtime configuration option. Additionally backtraces
  are now only captured when an embedder-generated trap actually reaches a
  WebAssembly call stack.
  [#4183](https://github.com/bytecodealliance/wasmtime/pull/4183)

* Usage of `*_unchecked` APIs for `Func` in the `wasmtime` crate and C API now
  take a `usize` parameter indicating the number of `ValRaw` values behind
  the associated pointer.
  [#4192](https://github.com/bytecodealliance/wasmtime/pull/4192)

### Fixed

* An improvement was made to the spill-slot allocation in code generation to fix
  an issue where some stack slots accidentally weren't reused. This issue was
  introduced with the landing of regalloc2 in 0.37.0 and may have resulted in
  larger-than-intended increases in stack frame sizes.
  [#4222](https://github.com/bytecodealliance/wasmtime/pull/4222)

--------------------------------------------------------------------------------

## 0.37.0

Released 2022-05-20

### Added

* Updated Cranelift to use regalloc2, a new register allocator. This should
  result in ~20% faster compile times, and for programs that suffered from
  register-allocation pressure before, up to ~20% faster generated code.
  [#3989](https://github.com/bytecodealliance/wasmtime/pull/3989)

* Pre-built binaries for macOS M1 machines are now available as release
  artifacts.
  [#3983](https://github.com/bytecodealliance/wasmtime/pull/3983)

* Copy-on-write images of memory can now be manually initialized for a `Module`
  with an explicit method call, but it is still not required to call this method
  and will automatically otherwise happen on the first instantiation.
  [#3964](https://github.com/bytecodealliance/wasmtime/pull/3964)

### Fixed

* Using `InstancePre::instantiate` or `Linker::instantiate` will now panic as
  intended when used with an async-configured `Store`.
  [#3972](https://github.com/bytecodealliance/wasmtime/pull/3972)

### Changed

* The unsafe `ValRaw` type in the `wasmtime` crate now always stores its values
  in little-endian format instead of the prior native-endian format. Users of
  `ValRaw` are recommended to audit their existing code for usage to continue
  working on big-endian platforms.
  [#4035](https://github.com/bytecodealliance/wasmtime/pull/4035)

### Removed

* Support for `Config::paged_memory_initialization` and the `uffd` crate feature
  have been removed from the `wasmtime` crate. Users should migrate to using
  `Config::memory_init_cow` which is more portable and faster at this point.
  [#4040](https://github.com/bytecodealliance/wasmtime/pull/4040)

--------------------------------------------------------------------------------

## 0.36.0

Released 2022-04-20

### Added

* Support for epoch-based interruption has been added to the C API.
  [#3925](https://github.com/bytecodealliance/wasmtime/pull/3925)

* Support for disabling libunwind-based backtraces of WebAssembly code at
  compile time has been added.
  [#3932](https://github.com/bytecodealliance/wasmtime/pull/3932)

* Async support for call hooks has been added to optionally execute "blocking"
  work whenever a wasm module is entered or exited relative to the host.
  [#3876](https://github.com/bytecodealliance/wasmtime/pull/3876)

### Fixed

* Loading a `Module` will now check, at runtime, that the compilation settings
  enabled in a `Config` are compatible with the native host. For example this
  ensures that if avx2 is enabled that the host actually has avx2 support.
  [#3899](https://github.com/bytecodealliance/wasmtime/pull/3899)

### Removed

* Support for `Config::interruptable` and `InterruptHandle` has been removed
  from the `wasmtime` crate. Users should migrate to using epoch-based
  interruption instead.
  [#3925](https://github.com/bytecodealliance/wasmtime/pull/3925)

* The module linking implementation of Wasmtime has been removed to make room
  for the upcoming support for the component model.
  [#3958](https://github.com/bytecodealliance/wasmtime/pull/3958)

--------------------------------------------------------------------------------

## 0.35.3

Released 2022-04-11.

### Fixed

* Backported a bugfix for an instruction lowering issue that could cause a
  regalloc panic due to an undefined register in some cases. No miscompilation
  was ever possible, but panics would result in a compilation failure.
  [#4012](https://github.com/bytecodealliance/wasmtime/pull/4012)

--------------------------------------------------------------------------------

## 0.35.2

Released 2022-03-31.

### Security Fixes

* [CVE-2022-24791](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-gwc9-348x-qwv2):
  Fixed a use after free with `externref`s and epoch interruption.

## 0.35.1

Released 2022-03-09.

### Fixed

* Fixed a bug in the x86-64 lowering of the `uextend` opcode for narrow (`i8`,
  `i16`) integer sources when the value is produced by one of several
  arithmetic instructions.
  [#3906](https://github.com/bytecodealliance/wasmtime/pull/3906)

## 0.35.0

Released 2022-03-07.

### Added

* The `wasmtime_wasi::add_to_linker` function now allows providing
  a context object of a custom type instead of `wasmtime_wasi::WasiCtx`,
  as long as that type implements the required WASI snapshot traits.
  This allows, for example, wrapping `WasiCtx` into a struct and providing
  custom implementations for those traits to override the default behaviour.

### Changed

* WebAssembly tables of `funcref` values are now lazily initialized which can,
  in some cases, greatly speed up instantiation of a module.
  [#3733](https://github.com/bytecodealliance/wasmtime/pull/3733)

* The `memfd` feature in 0.34.0, now renamed to `memory-init-cow`, has been
  enabled by default. This means that, where applicable, WebAssembly linear
  memories are now initialized with copy-on-write mappings. Support from this
  has been expanded from Linux-only to include macOS and other Unix systems when
  modules are loaded from precompiled `*.cwasm` files on disk.
  [#3777](https://github.com/bytecodealliance/wasmtime/pull/3777)
  [#3778](https://github.com/bytecodealliance/wasmtime/pull/3778)
  [#3787](https://github.com/bytecodealliance/wasmtime/pull/3787)
  [#3819](https://github.com/bytecodealliance/wasmtime/pull/3819)
  [#3831](https://github.com/bytecodealliance/wasmtime/pull/3831)

* Clarify that SSE 4.2 (and prior) is required for running WebAssembly code with
  simd support enabled on x86\_64.
  [#3816](https://github.com/bytecodealliance/wasmtime/pull/3816)
  [#3817](https://github.com/bytecodealliance/wasmtime/pull/3817)
  [#3833](https://github.com/bytecodealliance/wasmtime/pull/3833)
  [#3825](https://github.com/bytecodealliance/wasmtime/pull/3825)

* Support for profiling with VTune is now enabled at compile time by default,
  but it remains disabled at runtime by default.
  [#3821](https://github.com/bytecodealliance/wasmtime/pull/3821)

* The `ModuleLimits` type has been removed from the configuration of the pooling
  allocator in favor of configuring the total size of an instance allocation
  rather than each individual field.
  [#3837](https://github.com/bytecodealliance/wasmtime/pull/3837)

* The native stack size allowed for WebAssembly has been decreased from 1 MiB to
  512 KiB on all platforms to better accommodate running wasm on the main thread
  on Windows.
  [#3861](https://github.com/bytecodealliance/wasmtime/pull/3861)

* The `wasi-common` crate now supports doing polls for both read and write
  interest on a file descriptor at the same time.
  [#3866](https://github.com/bytecodealliance/wasmtime/pull/3866)

### Fixed

* The `Store::call_hook` callback is now invoked when entering host functions
  defined with `*_unchecked` variants.
  [#3881](https://github.com/bytecodealliance/wasmtime/pull/3881)

### Removed

* The incomplete and unmaintained ARM32 backend has been removed from Cranelift.
  [#3799](https://github.com/bytecodealliance/wasmtime/pull/3799)

--------------------------------------------------------------------------------

## 0.34.2

Released 2022-03-31.

### Security Fixes

* [CVE-2022-24791](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-gwc9-348x-qwv2):
  Fixed a use after free with `externref`s and epoch interruption.

## 0.34.1

Released 2022-02-16.

### Security Fixes

* [CVE-2022-23636](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-88xq-w8cq-xfg7):
  Fixed an invalid drop of a partially-initialized instance in the pooling instance
  allocator.

## 0.33.1

Released 2022-02-16.

### Security Fixes

* [CVE-2022-23636](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-88xq-w8cq-xfg7):
  Fixed an invalid drop of a partially-initialized instance in the pooling instance
  allocator.

## 0.34.0

Released 2022-02-07.

### Fixed

* The `wasi-common` default implementation of some attributes of files has been
  updated to ensure that `wasi-libc`'s `isatty` function works as intended.
  [#3696](https://github.com/bytecodealliance/wasmtime/pull/3696)

* A benign debug assertion related to `externref` and garbage-collection has
  been fixed.
  [#3734](https://github.com/bytecodealliance/wasmtime/pull/3734)

### Added

* Function names are now automatically demangled when informing profilers of
  regions of JIT code to apply Rust-specific demangling rules if applicable.
  [#3683](https://github.com/bytecodealliance/wasmtime/pull/3683)

* Support for profiling JIT-generated trampolines with VTune has been added.
  [#3687](https://github.com/bytecodealliance/wasmtime/pull/3687)

* Wasmtime now supports a new method of async preemption dubbed "epoch-based
  interruption" which is intended to be much more efficient than the current
  fuel-based method of preemption.
  [#3699](https://github.com/bytecodealliance/wasmtime/pull/3699)

* On Linux Wasmtime will now by default use copy-on-write mappings to initialize
  memories of wasm modules where possible, accelerating instantiation by
  avoiding costly memory copies. When combined with the pooling allocator this
  can also be used to speed up instance-reuse cases due to fewer syscalls to
  change memory mappings being necessary.
  [#3697](https://github.com/bytecodealliance/wasmtime/pull/3697)
  [#3738](https://github.com/bytecodealliance/wasmtime/pull/3738)
  [#3760](https://github.com/bytecodealliance/wasmtime/pull/3760)

* Wasmtime now supports the recently-added `sock_accept` WASI function.
  [#3711](https://github.com/bytecodealliance/wasmtime/pull/3711)

* Cranelift now has support for specifying blocks as cold.
  [#3698](https://github.com/bytecodealliance/wasmtime/pull/3698)

### Changed

* Many more instructions for the x64 backend have been migrated to ISLE,
  additionally with refactorings to make incorrect lowerings harder to
  accidentally write.
  [#3653](https://github.com/bytecodealliance/wasmtime/pull/3653)
  [#3659](https://github.com/bytecodealliance/wasmtime/pull/3659)
  [#3681](https://github.com/bytecodealliance/wasmtime/pull/3681)
  [#3686](https://github.com/bytecodealliance/wasmtime/pull/3686)
  [#3688](https://github.com/bytecodealliance/wasmtime/pull/3688)
  [#3690](https://github.com/bytecodealliance/wasmtime/pull/3690)
  [#3752](https://github.com/bytecodealliance/wasmtime/pull/3752)

* More instructions in the aarch64 backend are now lowered with ISLE.
  [#3658](https://github.com/bytecodealliance/wasmtime/pull/3658)
  [#3662](https://github.com/bytecodealliance/wasmtime/pull/3662)

* The s390x backend's lowering rules are now almost entirely defined with ISLE.
  [#3702](https://github.com/bytecodealliance/wasmtime/pull/3702)
  [#3703](https://github.com/bytecodealliance/wasmtime/pull/3703)
  [#3706](https://github.com/bytecodealliance/wasmtime/pull/3706)
  [#3717](https://github.com/bytecodealliance/wasmtime/pull/3717)
  [#3723](https://github.com/bytecodealliance/wasmtime/pull/3723)
  [#3724](https://github.com/bytecodealliance/wasmtime/pull/3724)

* Instantiation of modules in Wasmtime has been further optimized now that the
  copy-on-write memory initialization removed the previously most-expensive part
  of instantiating a module.
  [#3727](https://github.com/bytecodealliance/wasmtime/pull/3727)
  [#3739](https://github.com/bytecodealliance/wasmtime/pull/3739)
  [#3741](https://github.com/bytecodealliance/wasmtime/pull/3741)
  [#3742](https://github.com/bytecodealliance/wasmtime/pull/3742)

--------------------------------------------------------------------------------

## 0.33.0

Released 2022-01-05.

### Added

* Compiled wasm modules may now optionally omit debugging information about
  mapping addresses to source locations, resulting in smaller binaries.
  [#3598](https://github.com/bytecodealliance/wasmtime/pull/3598)

* The WebAssembly SIMD proposal is now enabled by default.
  [#3601](https://github.com/bytecodealliance/wasmtime/pull/3601)

--------------------------------------------------------------------------------

## 0.32.1

Released 2022-01-04.

### Fixed

* Cranelift: remove recently-added build dependency on `sha2` to allow usage in
  some dependency-sensitive environments, by computing ISLE manifest hashes
  with a different hash function.
  [#3619](https://github.com/bytecodealliance/wasmtime/pull/3619)

* Cranelift: fixed 8- and 16-bit behavior of popcount (bit population count)
  instruction. Does not affect Wasm frontend.
  [#3617](https://github.com/bytecodealliance/wasmtime/pull/3617)

* Cranelift: fixed miscompilation of 8- and 16-bit bit-rotate instructions.
  Does not affect Wasm frontend.
  [#3610](https://github.com/bytecodealliance/wasmtime/pull/3610)

--------------------------------------------------------------------------------

## 0.32.0

Released 2021-12-13.

### Added

* A new configuration option has been added to force using a "static" memory
  style to automatically limit growth of memories in some configurations.
  [#3503](https://github.com/bytecodealliance/wasmtime/pull/3503)

* The `InstancePre<T>` type now implements `Clone`.
  [#3510](https://github.com/bytecodealliance/wasmtime/pull/3510)

* Cranelift's instruction selection process has begun to be migrated towards the
  ISLE compiler and definition language.
  [#3506](https://github.com/bytecodealliance/wasmtime/pull/3506)

* A `pooling-allocator` feature has been added, which is on-by-default, to
  disable the pooling allocator at compile time.
  [#3514](https://github.com/bytecodealliance/wasmtime/pull/3514)

### Fixed

* A possible panic when parsing a WebAssembly `name` section has been fixed.
  [#3509](https://github.com/bytecodealliance/wasmtime/pull/3509)

* Generating native DWARF information for some C-produced modules has been
  fixed, notably those where there may be DWARF about dead code.
  [#3498](https://github.com/bytecodealliance/wasmtime/pull/3498)

* A number of SIMD code generation bugs have been fixed in the x64 backend
  by migrating their lowerings to ISLE.

--------------------------------------------------------------------------------

## 0.31.0

Released 2021-10-29.

### Added

* New `Func::new_unchecked` and `Func::call_unchecked` APIs have been added with
  accompanying functions in the C API to improve the performance of calls into
  wasm and the host in the C API.
  [#3350](https://github.com/bytecodealliance/wasmtime/pull/3350)

* Release binaries are now available for the s390x-unknown-linux-gnu
  architecture.
  [#3372](https://github.com/bytecodealliance/wasmtime/pull/3372)

* A new `ResourceLimiterAsync` trait is added which allows asynchronous blocking
  of WebAssembly on instructions such as `memory.grow`.
  [#3393](https://github.com/bytecodealliance/wasmtime/pull/3393)

### Changed

* The `Func::call` method now takes a slice to write the results into rather
  than returning a boxed slice.
  [#3319](https://github.com/bytecodealliance/wasmtime/pull/3319)

* Trampolines are now covered when jitdump profiling is enabled.
  [#3344](https://github.com/bytecodealliance/wasmtime/pull/3344)

### Fixed

* Debugging with GDB has been fixed on Windows.
  [#3373](https://github.com/bytecodealliance/wasmtime/pull/3373)

* Some quadradic behavior in Wasmtime's compilation of modules has been fixed.
  [#3469](https://github.com/bytecodealliance/wasmtime/pull/3469)
  [#3466](https://github.com/bytecodealliance/wasmtime/pull/3466)

* Bounds-checks for wasm memory accesses in certain non-default configurations
  have been fixed to correctly allow loads at the end of the address space.
  [#3462](https://github.com/bytecodealliance/wasmtime/pull/3462)

* When type-checking memories and tables for satisfying instance imports the
  runtime size of the table/memory is now consulted instead of the object's
  original type.
  [#3450](https://github.com/bytecodealliance/wasmtime/pull/3450)

### Removed

* The Lightbeam backend has been removed, as per [RFC 14].
  [#3390](https://github.com/bytecodealliance/wasmtime/pull/3390)

[RFC 14]: https://github.com/bytecodealliance/rfcs/pull/14

* Cranelift's old x86 backend has been removed, as per [RFC 12].
  [#3309](https://github.com/bytecodealliance/wasmtime/pull/3009)

[RFC 12]: https://github.com/bytecodealliance/rfcs/pull/12

## 0.30.0

Released 2021-09-17.

### Security Fixes

* [CVE-2021-39216](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-v4cp-h94r-m7xf):
  Fixed a use after free passing `externref`s to Wasm in Wasmtime.

* [CVE-2021-39218](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-4873-36h9-wv49):
  Fixed an out-of-bounds read/write and invalid free with `externref`s and GC
  safepoints in Wasmtime.

* [CVE-2021-39219](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-q879-9g95-56mx):
  Fixed a bug where using two different `Engine`s with the same `Linker`-define
  functions caused unsafety without `unsafe` blocks.

### Added

* Added experimental support for the in-progress 64-bit memories Wasm proposal.

* Added support to build Wasmtime without the compiler. This lets you run
  pre-compiled Wasm modules, without the ability (or potential attack surface)
  of compiling new Wasm modules. The compilation functionality is gated by the
  on-by-default `cranelift` cargo feature.

* Added support for NaN canonicalization with SIMD vectors.

* Added support for differential fuzzing against V8's Wasm engine.

* Added support for fuzzing against the Wasm spec interpreter.

* Enabled SIMD fuzzing on oss-fuzz.

### Changed

* A variety of performance improvements to loading pre-compiled modules.

* A variety of performance improvements to function calls, both through Rust and
  the C API.

* Leaf functions that do not use the stack no longer bump the frame pointer on
  aarch64 and s390x.

* Many updates and expanded instruction support to the in-progress CLIF
  interpreter.

* Expanded fuzzing of reference types and GC.

### Fixed

* A number of fixes to both aarch64 and x86_64 support for the Wasm SIMD
  proposal and the underlying CLIF vector instructions.

* Fixed a potential infinite loop in the SSA computation for
  `cranelift-frontend`. This was not reachable from `cranelift-wasm` or
  Wasmtime, but might have affected general Cranelift users.

### Removed

* The `wasmtime wasm2obj` subcommand has been removed. Generating raw object
  files for linking natively is no longer supported. Use the `wasmtime compile`
  subcommand to pre-compile a Wasm module and `wasmtime run` to run pre-compiled
  Wasm modules.

## 0.29.0

Released 2021-08-02.

### Changed

* Instance exports are now loaded lazily from instances instead of eagerly as
  they were before. This is an internal-only change and is not a breaking
  change.
  [#2984](https://github.com/bytecodealliance/wasmtime/pull/2984)

* All linear memories created by Wasmtime will now, by default, have guard pages
  in front of them in addition to after them. This is intended to help mitigate
  future bugs in Cranelift, should they arise.
  [#2977](https://github.com/bytecodealliance/wasmtime/pull/2977)

* Linear memories now correctly support a maximum size of 4GB. Previously, the
  limit field was 32 bits, which did not properly support a full 4GB memory.
  This update is also a necessary change in preparation for future memory64
  support.
  [#3013](https://github.com/bytecodealliance/wasmtime/pull/3013)
  [#3134](https://github.com/bytecodealliance/wasmtime/pull/3134)

* Injection counts of fuel into a `wasmtime::Store` now uses a u64 instead of a
  u32.
  [#3048](https://github.com/bytecodealliance/wasmtime/pull/3048)

### Added

* Support for `i128` has improved in the AArch64 backend.
  [#2959](https://github.com/bytecodealliance/wasmtime/pull/2959)
  [#2975](https://github.com/bytecodealliance/wasmtime/pull/2975)
  [#2985](https://github.com/bytecodealliance/wasmtime/pull/2985)
  [#2990](https://github.com/bytecodealliance/wasmtime/pull/2990)
  [#3002](https://github.com/bytecodealliance/wasmtime/pull/3002)
  [#3004](https://github.com/bytecodealliance/wasmtime/pull/3004)
  [#3005](https://github.com/bytecodealliance/wasmtime/pull/3005)
  [#3008](https://github.com/bytecodealliance/wasmtime/pull/3008)
  [#3027](https://github.com/bytecodealliance/wasmtime/pull/3027)

* The s390x backend now supports z14 and atomics.
  [#2988](https://github.com/bytecodealliance/wasmtime/pull/2988)
  [#2991](https://github.com/bytecodealliance/wasmtime/pull/2991)

* The `wasmtime::Linker` type now implements `Clone`.
  [#2993](https://github.com/bytecodealliance/wasmtime/pull/2993)

* Support for the SIMD proposal on both x86\_64 and AArch64 has improved. On
  x86\_64, all SIMD opcodes are now supported.
  [#2997](https://github.com/bytecodealliance/wasmtime/pull/2997)
  [#3035](https://github.com/bytecodealliance/wasmtime/pull/3035)
  [#2982](https://github.com/bytecodealliance/wasmtime/pull/2982)
  [#3084](https://github.com/bytecodealliance/wasmtime/pull/3084)
  [#3082](https://github.com/bytecodealliance/wasmtime/pull/3082)
  [#3107](https://github.com/bytecodealliance/wasmtime/pull/3107)
  [#3105](https://github.com/bytecodealliance/wasmtime/pull/3105)
  [#3114](https://github.com/bytecodealliance/wasmtime/pull/3114)
  [#3070](https://github.com/bytecodealliance/wasmtime/pull/3070)
  [#3126](https://github.com/bytecodealliance/wasmtime/pull/3126)

* A `Trap` can now display its reason without also displaying the backtrace.
  [#3033](https://github.com/bytecodealliance/wasmtime/pull/3033)

* An initiall fuzzer for CLIF has been added.
  [#3038](https://github.com/bytecodealliance/wasmtime/pull/3038)

* High-level architecture documentation has been added for Wasmtime.
  [#3019](https://github.com/bytecodealliance/wasmtime/pull/3019)

* Support for multi-memory can now be configured in Wasmtime's C API.
  [#3071](https://github.com/bytecodealliance/wasmtime/pull/3071)

* The `wasmtime` crate now supports a `posix-signals-on-macos` feature to force
  the usage of signals instead of mach ports to handle traps on macOS.
  [#3063](https://github.com/bytecodealliance/wasmtime/pull/3063)

* Wasmtime's C API now has a `wasmtime_trap_code` function to get the raw trap
  code, if present, for a trap.
  [#3086](https://github.com/bytecodealliance/wasmtime/pull/3086)

* Wasmtime's C API now has a `wasmtime_linker_define_func` function to define a
  store-independent function within a linker.
  [#3122](https://github.com/bytecodealliance/wasmtime/pull/3122)

* A `wasmtime::Linker::module_async` function was added as the asynchronous
  counterpart to `wasmtime::Linker::module`.
  [#3121](https://github.com/bytecodealliance/wasmtime/pull/3121)

### Fixed

* Compiling the `wasmtime` crate into a `dylib` crate type has been fixed.
  [#3010](https://github.com/bytecodealliance/wasmtime/pull/3010)

* The enter/exit hooks for WebAssembly are now executed for an instance's
  `start` function, if present.
  [#3001](https://github.com/bytecodealliance/wasmtime/pull/3001)

* Some WASI functions in `wasi-common` have been fixed for big-endian platforms.
  [#3016](https://github.com/bytecodealliance/wasmtime/pull/3016)

* Wasmtime no longer erroneously assumes that all custom sections may contain
  DWARF information, reducing instances of `Trap`'s `Display` implementation
  providing misleading information to set an env var to get more information.
  [#3083](https://github.com/bytecodealliance/wasmtime/pull/3083)

* Some issues with parsing DWARF debug information have been fixed.
  [#3116](https://github.com/bytecodealliance/wasmtime/pull/3116)

## 0.28.0

Released 2021-06-09.

### Changed

* Breaking: Wasmtime's embedding API has been redesigned, as specified in [RFC
  11]. Rust users can now enjoy easier times with `Send` and `Sync`, and all
  users can now more clearly manage memory, especially in the C API. Language
  embeddings have been updated to the new API as well.
  [#2897](https://github.com/bytecodealliance/wasmtime/pull/2897)

[RFC 11]: https://github.com/bytecodealliance/rfcs/pull/11

### Added

* A new `InstancePre` type, created with `Linker::instantiate_pre`, has been
  added to perform type-checking of an instance once and reduce the work done
  for each instantiation of a module:
  [#2962](https://github.com/bytecodealliance/wasmtime/pull/2962)

* Deserialization of a module can now optionally skip checking the wasmtime
  version string:
  [#2945](https://github.com/bytecodealliance/wasmtime/pull/2945)

* A method has been exposed to frontload per-thread initialization costs if the
  latency of every last wasm call is important:
  [#2946](https://github.com/bytecodealliance/wasmtime/pull/2946)

* Hooks have been added for entry/exit into wasm code to allow embeddings to
  track time and other properties about execution in a wasm environment:
  [#2952](https://github.com/bytecodealliance/wasmtime/pull/2952)

* A [C++ embedding of Wasmtime has been written][cpp].

[RFC 11]: https://github.com/bytecodealliance/rfcs/pull/11
[cpp]: https://github.com/bytecodealliance/wasmtime-cpp

### Fixed

* Multiple returns on macOS AArch64 have been fixed:
  [#2956](https://github.com/bytecodealliance/wasmtime/pull/2956)

## 0.27.0

Released 2021-05-21.

### Security Fixes

* Fixed a security issue in Cranelift's x64 backend that could result in a heap
  sandbox escape due to an incorrect sign-extension:
  [#2913](https://github.com/bytecodealliance/wasmtime/issues/2913).

### Added

* Support for IBM z/Architecture (`s390x`) machines in Cranelift and Wasmtime:
  [#2836](https://github.com/bytecodealliance/wasmtime/pull/2836),
  [#2837](https://github.com/bytecodealliance/wasmtime/pull/2837),
  [#2838](https://github.com/bytecodealliance/wasmtime/pull/2838),
  [#2843](https://github.com/bytecodealliance/wasmtime/pull/2843),
  [#2854](https://github.com/bytecodealliance/wasmtime/pull/2854),
  [#2870](https://github.com/bytecodealliance/wasmtime/pull/2870),
  [#2871](https://github.com/bytecodealliance/wasmtime/pull/2871),
  [#2872](https://github.com/bytecodealliance/wasmtime/pull/2872),
  [#2874](https://github.com/bytecodealliance/wasmtime/pull/2874).

* Improved async support in wasi-common runtime:
  [#2832](https://github.com/bytecodealliance/wasmtime/pull/2832).

* Added `Store::with_limits`, `StoreLimits`, and `ResourceLimiter` to the
  Wasmtime API to help with enforcing resource limits at runtime. The
  `ResourceLimiter` trait can be implemented by custom resource limiters to
  decide if linear memories or tables can be grown.

* Added `allow-unknown-exports` option for the run command:
  [#2879](https://github.com/bytecodealliance/wasmtime/pull/2879).

* Added API to notify that a `Store` has moved to a new thread:
  [#2822](https://github.com/bytecodealliance/wasmtime/pull/2822).

* Documented guidance around using Wasmtime in multithreaded contexts:
  [#2812](https://github.com/bytecodealliance/wasmtime/pull/2812).
  In the future, the Wasmtime API will change to allow some of its core types
  to be Send/Sync; see the in-progress
  [#2897](https://github.com/bytecodealliance/wasmtime/pull/2897) for details.

* Support calls from native code to multiple-return-value functions:
  [#2806](https://github.com/bytecodealliance/wasmtime/pull/2806).

### Changed

* Breaking: `Memory::new` has been changed to return `Result` as creating a
  host memory object is now a fallible operation when the initial size of
  the memory exceeds the store limits.

### Fixed

* Many instruction selection improvements on x64 and aarch64:
  [#2819](https://github.com/bytecodealliance/wasmtime/pull/2819),
  [#2828](https://github.com/bytecodealliance/wasmtime/pull/2828),
  [#2823](https://github.com/bytecodealliance/wasmtime/pull/2823),
  [#2862](https://github.com/bytecodealliance/wasmtime/pull/2862),
  [#2886](https://github.com/bytecodealliance/wasmtime/pull/2886),
  [#2889](https://github.com/bytecodealliance/wasmtime/pull/2889),
  [#2905](https://github.com/bytecodealliance/wasmtime/pull/2905).

* Improved performance of Wasmtime runtime substantially:
  [#2811](https://github.com/bytecodealliance/wasmtime/pull/2811),
  [#2818](https://github.com/bytecodealliance/wasmtime/pull/2818),
  [#2821](https://github.com/bytecodealliance/wasmtime/pull/2821),
  [#2847](https://github.com/bytecodealliance/wasmtime/pull/2847),
  [#2900](https://github.com/bytecodealliance/wasmtime/pull/2900).

* Fixed WASI issue with file metadata on Windows:
  [#2884](https://github.com/bytecodealliance/wasmtime/pull/2884).

* Fixed an issue with debug info and an underflowing (trapping) offset:
  [#2866](https://github.com/bytecodealliance/wasmtime/pull/2866).

* Fixed an issue with unwind information in the old x86 backend:
  [#2845](https://github.com/bytecodealliance/wasmtime/pull/2845).

* Fixed i32 spilling in x64 backend:
  [#2840](https://github.com/bytecodealliance/wasmtime/pull/2840).

## 0.26.0

Released 2021-04-05.

### Added

* Added the `wasmtime compile` command to support AOT compilation of Wasm
  modules. This adds the `Engine::precompile_module` method. Also added the
  `Config::target` method to change the compilation target of the
  configuration. This can be used in conjunction with
  `Engine::precompile_module` to target a different host triple than the
  current one.
  [#2791](https://github.com/bytecodealliance/wasmtime/pull/2791)

* Support for macOS on aarch64 (Apple M1 Silicon), including Apple-specific
  calling convention details and unwinding/exception handling using Mach ports.
  [#2742](https://github.com/bytecodealliance/wasmtime/pull/2742),
  [#2723](https://github.com/bytecodealliance/wasmtime/pull/2723)

* A number of SIMD instruction implementations in the new x86-64 backend.
  [#2771](https://github.com/bytecodealliance/wasmtime/pull/2771)

* Added the `Config::cranelift_flag_enable` method to enable setting Cranelift
  boolean flags or presets in a config.

* Added CLI option `--cranelift-enable` to enable boolean settings and ISA presets.

* Deduplicate function signatures in Wasm modules.
  [#2772](https://github.com/bytecodealliance/wasmtime/pull/2772)

* Optimize overheads of calling into Wasm functions.
  [#2757](https://github.com/bytecodealliance/wasmtime/pull/2757),
  [#2759](https://github.com/bytecodealliance/wasmtime/pull/2759)

* Improvements related to Module Linking: compile fewer trampolines;

  [#2774](https://github.com/bytecodealliance/wasmtime/pull/2774)

* Re-export sibling crates from `wasmtime-wasi` to make embedding easier
  without needing to match crate versions.
  [#2776](https://github.com/bytecodealliance/wasmtime/pull/2776)

### Changed

* Switched the default compiler backend on x86-64 to Cranelift's new backend.
  This should not have any user-visible effects other than possibly runtime
  performance improvements. The old backend is still available with the
  `old-x86-backend` feature flag to the `cranelift-codegen` or `wasmtime`
  crates, or programmatically with `BackendVariant::Legacy`. We plan to
  maintain the old backend for at least one more release and ensure it works on
  CI.
  [#2718](https://github.com/bytecodealliance/wasmtime/pull/2718)

* Breaking: `Module::deserialize` has been removed in favor of `Module::new`.

* Breaking: `Config::cranelift_clear_cpu_flags` was removed. Use `Config::target`
  to clear the CPU flags for the host's target.

* Breaking: `Config::cranelift_other_flag` was renamed to `Config::cranelift_flag_set`.

* CLI changes:
  * Wasmtime CLI options to enable WebAssembly features have been replaced with
    a singular `--wasm-features` option. The previous options are still
    supported, but are not displayed in help text.
  * Breaking: the CLI option `--cranelift-flags` was changed to
    `--cranelift-set`.
  * Breaking: the CLI option `--enable-reference-types=false` has been changed
    to `--wasm-features=-reference-types`.
  * Breaking: the CLI option `--enable-multi-value=false` has been changed to
    `--wasm-features=-multi-value`.
  * Breaking: the CLI option `--enable-bulk-memory=false` has been changed to
    `--wasm-features=-bulk-memory`.

* Improved error-reporting in wiggle.
  [#2760](https://github.com/bytecodealliance/wasmtime/pull/2760)

* Make WASI sleeping fallible (some systems do not support sleep).
  [#2756](https://github.com/bytecodealliance/wasmtime/pull/2756)

* WASI: Support `poll_oneoff` with a sleep.
  [#2753](https://github.com/bytecodealliance/wasmtime/pull/2753)

* Allow a `StackMapSink` to be passed when defining functions with
  `cranelift-module`.
  [#2739](https://github.com/bytecodealliance/wasmtime/pull/2739)

* Some refactoring in new x86-64 backend to prepare for VEX/EVEX (e.g.,
  AVX-512) instruction encodings to be supported.
  [#2799](https://github.com/bytecodealliance/wasmtime/pull/2799)

### Fixed

* Fixed a corner case in `srem` (signed remainder) in the new x86-64 backend:
  `INT_MIN % -1` should return `0`, rather than trapping. This only occurred
  when `avoid_div_traps == false` was set by the embedding.
  [#2763](https://github.com/bytecodealliance/wasmtime/pull/2763)

* Fixed a memory leak of the `Store` when an instance traps.
  [#2803](https://github.com/bytecodealliance/wasmtime/pull/2803)

* Some fuzzing-related fixes.
  [#2788](https://github.com/bytecodealliance/wasmtime/pull/2788),
  [#2770](https://github.com/bytecodealliance/wasmtime/pull/2770)

* Fixed memory-initialization bug in uffd allocator that could copy into the
  wrong destination under certain conditions. Does not affect the default
  wasmtime instance allocator.
  [#2801](https://github.com/bytecodealliance/wasmtime/pull/2801)

* Fix printing of float values from the Wasmtime CLI.
  [#2797](https://github.com/bytecodealliance/wasmtime/pull/2797)

* Remove the ability for the `Linker` to instantiate modules with duplicate
  import strings of different types.
  [#2789](https://github.com/bytecodealliance/wasmtime/pull/2789)

## 0.25.0

Released 2021-03-16.

### Added

* An implementation of a pooling instance allocator, optionally backed by
  `userfaultfd` on Linux, was added to improve the performance of embeddings
  that instantiate a large number of instances continuously.
  [#2518](https://github.com/bytecodealliance/wasmtime/pull/2518)

* Host functions can now be defined on `Config` to share the function across all
  `Store` objects connected to an `Engine`. This can improve the time it takes
  to instantiate instances in a short-lived `Store`.
  [#2625](https://github.com/bytecodealliance/wasmtime/pull/2625)

* The `Store` object now supports having typed values attached to it which can
  be retrieved from host functions.
  [#2625](https://github.com/bytecodealliance/wasmtime/pull/2625)

* The `wiggle` code generator now supports `async` host functions.
  [#2701](https://github.com/bytecodealliance/wasmtime/pull/2701)

### Changed

* The `Func::getN{,_async}` APIs have all been removed in favor of a new
  `Func::typed` API which should be more compact in terms of API surface area as
  well as more flexible in how it can be used.
  [#2719](https://github.com/bytecodealliance/wasmtime/pull/2719)

* `Engine::new` has been changed from returning `Engine` to returning
  `anyhow::Result<Engine>`. Callers of `Engine::new` will need to be updated to
  use the `?` operator on the return value or otherwise unwrap the result to get
  the `Engine`.

### Fixed

* Interpretation of timestamps in `poll_oneoff` for WASI have been fixed to
  correctly use nanoseconds instead of microseconds.
  [#2717](https://github.com/bytecodealliance/wasmtime/pull/2717)

## 0.24.0

Released 2021-03-04.

### Added

* Implement support for `async` functions in Wasmtime
  [#2434](https://github.com/bytecodealliance/wasmtime/pull/2434)

### Fixed

* Fix preservation of the sigaltstack on macOS
  [#2676](https://github.com/bytecodealliance/wasmtime/pull/2676)
* Fix incorrect semver dependencies involving fs-set-times.
  [#2705](https://github.com/bytecodealliance/wasmtime/pull/2705)
* Fix some `i128` shift-related bugs in x64 backend.
  [#2682](https://github.com/bytecodealliance/wasmtime/pull/2682)
* Fix incomplete trap metadata due to multiple traps at one address
  [#2685](https://github.com/bytecodealliance/wasmtime/pull/2685)

## 0.23.0

Released 2021-02-16.

### Added

* Support for limiting WebAssembly execution with fuel was added, including
  support in the C API.
  [#2611](https://github.com/bytecodealliance/wasmtime/pull/2611)
  [#2643](https://github.com/bytecodealliance/wasmtime/pull/2643)
* Wasmtime now has more knobs for limiting memory and table allocations
  [#2617](https://github.com/bytecodealliance/wasmtime/pull/2617)
* Added a method to share `Config` across machines
  [#2608](https://github.com/bytecodealliance/wasmtime/pull/2608)
* Added a safe memory read/write API
  [#2528](https://github.com/bytecodealliance/wasmtime/pull/2528)
* Added support for the experimental wasi-crypto APIs
  [#2597](https://github.com/bytecodealliance/wasmtime/pull/2597)
* Added an instance limit to `Config`
  [#2593](https://github.com/bytecodealliance/wasmtime/pull/2593)
* Implemented module-linking's outer module aliases
  [#2590](https://github.com/bytecodealliance/wasmtime/pull/2590)
* Cranelift now supports 128-bit operations for the new x64 backend.
  [#2539](https://github.com/bytecodealliance/wasmtime/pull/2539)
* Cranelift now has detailed debug-info (DWARF) support in new backends (initially x64).
  [#2565](https://github.com/bytecodealliance/wasmtime/pull/2565)
* Cranelift now uses the `POPCNT`, `TZCNT`, and `LZCNT`, as well as SSE 4.1
  rounding instructions on x64 when available.
* Cranelift now uses the `CNT`, instruction on aarch64 when available.

### Changed

* A new WASI implementation built on the new
  [`cap-std`](https://github.com/bytecodealliance/cap-std) crate was added,
  replacing the previous implementation. This brings improved robustness,
  portability, and performance.

* `wasmtime_wasi::WasiCtxBuilder` moved to
  `wasi_cap_std_sync::WasiCtxBuilder`.

* The WebAssembly C API is updated, with a few minor API changes
  [#2579](https://github.com/bytecodealliance/wasmtime/pull/2579)

### Fixed

* Fixed a panic in WASI `fd_readdir` on large directories
  [#2620](https://github.com/bytecodealliance/wasmtime/pull/2620)
* Fixed a memory leak with command modules
  [#2017](https://github.com/bytecodealliance/wasmtime/pull/2017)

--------------------------------------------------------------------------------

## 0.22.0

Released 2021-01-07.

### Added

* Experimental support for [the module-linking
  proposal](https://github.com/WebAssembly/module-linking) was
  added. [#2094](https://github.com/bytecodealliance/wasmtime/pull/2094)

* Added support for [the reference types
  proposal](https://webassembly.github.io/reference-types) on the aarch64
  architecture. [#2410](https://github.com/bytecodealliance/wasmtime/pull/2410)

* Experimental support for [wasi-nn](https://github.com/WebAssembly/wasi-nn) was
  added. [#2208](https://github.com/bytecodealliance/wasmtime/pull/2208)

### Changed

### Fixed

* Fixed an issue where the `select` instruction didn't accept `v128` SIMD
  operands. [#2391](https://github.com/bytecodealliance/wasmtime/pull/2391)

* Fixed an issue where Wasmtime could potentially use the wrong stack map during
  GCs, leading to a
  panic. [#2396](https://github.com/bytecodealliance/wasmtime/pull/2396)

* Fixed an issue where if a host-defined function erroneously returned a value
  from a different store, that value would be
  leaked. [#2424](https://github.com/bytecodealliance/wasmtime/pull/2424)

* Fixed a bug where in certain cases if a module's instantiation failed, it
  could leave trampolines in the store that referenced the no-longer-valid
  instance. These trampolines could be reused in future instantiations, leading
  to use after free bugs.
  [#2408](https://github.com/bytecodealliance/wasmtime/pull/2408)

* Fixed a miscompilation on aarch64 where certain instructions would read `SP`
  instead of the zero register. This could only affect you if you explicitly
  enabled the Wasm SIMD
  proposal. [#2548](https://github.com/bytecodealliance/wasmtime/pull/2548)

--------------------------------------------------------------------------------

## 0.21.0

Released 2020-11-05.

### Added

* Experimental support for the multi-memory proposal was added.
  [#2263](https://github.com/bytecodealliance/wasmtime/pull/2263)

* The `Trap::trap_code` API enables learning what kind of trap was raised.
  [#2309](https://github.com/bytecodealliance/wasmtime/pull/2309)

### Changed

* WebAssembly module validation is now parallelized.
  [#2059](https://github.com/bytecodealliance/wasmtime/pull/2059)

* Documentation is now available at docs.wasmtime.dev.
  [#2317](https://github.com/bytecodealliance/wasmtime/pull/2317)

* Windows now compiles like other platforms with a huge guard page instead of
  having its own custom limit which made modules compile and run more slowly.
  [#2326](https://github.com/bytecodealliance/wasmtime/pull/2326)

* The size of the cache entry for serialized modules has been greatly reduced.
  [#2321](https://github.com/bytecodealliance/wasmtime/pull/2321)
  [#2322](https://github.com/bytecodealliance/wasmtime/pull/2322)
  [#2324](https://github.com/bytecodealliance/wasmtime/pull/2324)
  [#2325](https://github.com/bytecodealliance/wasmtime/pull/2325)

* The `FuncType` API constructor and accessors are now iterator-based.
  [#2365](https://github.com/bytecodealliance/wasmtime/pull/2365)

### Fixed

* A panic in compiling reference-types-using modules has been fixed.
  [#2350](https://github.com/bytecodealliance/wasmtime/pull/2350)

--------------------------------------------------------------------------------

## 0.20.0

Released 2020-09-23.

### Added

* Support for explicitly serializing and deserializing compiled wasm modules has
  been added.
  [#2020](https://github.com/bytecodealliance/wasmtime/pull/2020)

* A `wasmtime_store_gc` C API was added to run GC for `externref`.
  [#2052](https://github.com/bytecodealliance/wasmtime/pull/2052)

* Support for atomics in Cranelift has been added. Support is not fully
  implemented in Wasmtime at this time, however.
  [#2077](https://github.com/bytecodealliance/wasmtime/pull/2077)

* The `Caller::get_export` function is now implemented for `Func` references as
  well.
  [#2108](https://github.com/bytecodealliance/wasmtime/pull/2108)

### Fixed

* Leaks in the C API have been fixed.
  [#2040](https://github.com/bytecodealliance/wasmtime/pull/2040)

* The `wasm_val_copy` C API has been fixed for reference types.
  [#2041](https://github.com/bytecodealliance/wasmtime/pull/2041)

* Fix a panic with `Func::new` and reference types when the store doesn't have
  reference types enabled.
  [#2039](https://github.com/bytecodealliance/wasmtime/pull/2039)

--------------------------------------------------------------------------------

## 0.19.0

Released 2020-07-14.

### Added

* The [WebAssembly reference-types proposal][reftypes] is now supported in
  Wasmtime and the C API.
  [#1832](https://github.com/bytecodealliance/wasmtime/pull/1832),
  [#1882](https://github.com/bytecodealliance/wasmtime/pull/1882),
  [#1894](https://github.com/bytecodealliance/wasmtime/pull/1894),
  [#1901](https://github.com/bytecodealliance/wasmtime/pull/1901),
  [#1923](https://github.com/bytecodealliance/wasmtime/pull/1923),
  [#1969](https://github.com/bytecodealliance/wasmtime/pull/1969),
  [#1973](https://github.com/bytecodealliance/wasmtime/pull/1973),
  [#1982](https://github.com/bytecodealliance/wasmtime/pull/1982),
  [#1984](https://github.com/bytecodealliance/wasmtime/pull/1984),
  [#1991](https://github.com/bytecodealliance/wasmtime/pull/1991),
  [#1996](https://github.com/bytecodealliance/wasmtime/pull/1996)

* The [WebAssembly simd proposal's][simd] spec tests now pass in Wasmtime.
  [#1765](https://github.com/bytecodealliance/wasmtime/pull/1765),
  [#1876](https://github.com/bytecodealliance/wasmtime/pull/1876),
  [#1941](https://github.com/bytecodealliance/wasmtime/pull/1941),
  [#1957](https://github.com/bytecodealliance/wasmtime/pull/1957),
  [#1990](https://github.com/bytecodealliance/wasmtime/pull/1990),
  [#1994](https://github.com/bytecodealliance/wasmtime/pull/1994)

* Wasmtime can now be compiled without the usage of threads for parallel
  compilation, although this is still enabled by default.
  [#1903](https://github.com/bytecodealliance/wasmtime/pull/1903)

* The C API is [now
  documented](https://bytecodealliance.github.io/wasmtime/c-api/).
  [#1928](https://github.com/bytecodealliance/wasmtime/pull/1928),
  [#1959](https://github.com/bytecodealliance/wasmtime/pull/1959),
  [#1968](https://github.com/bytecodealliance/wasmtime/pull/1968)

* A `wasmtime_linker_get_one_by_name` function was added to the C API.
  [#1897](https://github.com/bytecodealliance/wasmtime/pull/1897)

* A `wasmtime_trap_exit_status` function was added to the C API.
  [#1912](https://github.com/bytecodealliance/wasmtime/pull/1912)

* Compilation for the `aarch64-linux-android` target should now work, although
  keep in mind this platform is not fully tested still.
  [#2002](https://github.com/bytecodealliance/wasmtime/pull/2002)

[reftypes]: https://github.com/WebAssembly/reference-types

### Fixed

* Runtime warnings when using Wasmtime on musl have been fixed.
  [#1914](https://github.com/bytecodealliance/wasmtime/pull/1914)

* A bug affecting Windows unwind information with functions that have spilled
  floating point registers has been fixed.
  [#1983](https://github.com/bytecodealliance/wasmtime/pull/1983)

### Changed

* Wasmtime's default branch and development now happens on the `main` branch
  instead of `master`.
  [#1924](https://github.com/bytecodealliance/wasmtime/pull/1924)

### Removed

* The "host info" support in the C API has been removed since it was never fully
  or correctly implemented.
  [#1922](https://github.com/bytecodealliance/wasmtime/pull/1922)

* Support for the `*_same` functions in the C API has been removed in the same
  vein as the host info APIs.
  [#1926](https://github.com/bytecodealliance/wasmtime/pull/1926)

--------------------------------------------------------------------------------

## 0.18.0

Release 2020-06-09.

### Added

The `WasmTy` trait is now implemented for `u32` and `u64`.

  [#1808](https://github.com/bytecodealliance/wasmtime/pull/1808)

--------------------------------------------------------------------------------

## 0.17.0

Released 2020-06-01.

### Added

* The [Commands and Reactors ABI] is now supported in the Rust API. `Linker::module`
  loads a module and automatically handles Commands and Reactors semantics.

  [#1565](https://github.com/bytecodealliance/wasmtime/pull/1565)

[Commands and Reactors ABI]: https://github.com/WebAssembly/WASI/blob/master/design/application-abi.md#current-unstable-abi

The `Table::grow` function now returns the previous table size, making it consistent
with the `table.grow` instruction.

  [#1653](https://github.com/bytecodealliance/wasmtime/pull/1653)

New Wasmtime-specific C APIs for working with tables were added which provide more
detailed error information and which make growing a table more consistent with the
`table.grow` instruction as well.

  [#1654](https://github.com/bytecodealliance/wasmtime/pull/1654)

The C API now includes support for enabling logging in Wasmtime.

  [#1737](https://github.com/bytecodealliance/wasmtime/pull/1737)

### Changed

The WASI `proc_exit` function no longer exits the host process. It now unwinds the
callstack back to the wasm entrypoint, and the exit value is available from the
`Trap::i32_exit_status` method.

  [#1646](https://github.com/bytecodealliance/wasmtime/pull/1646)

The WebAssembly [multi-value](https://github.com/WebAssembly/multi-value/) proposal
is now enabled by default.

  [#1667](https://github.com/bytecodealliance/wasmtime/pull/1667)

The Rust API does not require a store provided during `Module::new` operation. The `Module` can be send across threads and instantiate for a specific store. The `Instance::new` now requires the store.

  [#1761](https://github.com/bytecodealliance/wasmtime/pull/1761)

--------------------------------------------------------------------------------

## 0.16.0

Released 2020-04-29.

### Added

* The `Instance` struct has new accessors, `get_func`, `get_table`,
  `get_memory`, and `get_global` for quickly looking up exported
  functions, tables, memories, and globals by name.
  [#1524](https://github.com/bytecodealliance/wasmtime/pull/1524)

* The C API has a number of new `wasmtime_*` functions which return error
  objects to get detailed error information when an API fails.
  [#1467](https://github.com/bytecodealliance/wasmtime/pull/1467)

* Users now have fine-grained control over creation of instances of `Memory`
  with a new `MemoryCreator` trait.
  [#1400](https://github.com/bytecodealliance/wasmtime/pull/1400)

* Go bindings for Wasmtime are [now available][go-bindings].
  [#1481](https://github.com/bytecodealliance/wasmtime/pull/1481)

* APIs for looking up values in a `Linker` have been added.
  [#1480](https://github.com/bytecodealliance/wasmtime/pull/1480)

* Preliminary support for AArch64, also known as ARM64.
  [#1581](https://github.com/bytecodealliance/wasmtime/pull/1581)

[go-bindings]: https://github.com/bytecodealliance/wasmtime-go

### Changed

* `Instance::exports` now returns `Export` objects which contain
  the `name`s of the exports in addition to their `Extern` definitions,
  so it's no longer necessary to use `Module::exports` to obtain the
  export names.
  [#1524](https://github.com/bytecodealliance/wasmtime/pull/1524)

* The `Func::call` API has changed its error type from `Trap` to `anyhow::Error`
  to distinguish between wasm traps and runtime violations (like the wrong
  number of parameters).
  [#1467](https://github.com/bytecodealliance/wasmtime/pull/1467)

* A number of `wasmtime_linker_*` and `wasmtime_config_*` C APIs have new type
  signatures which reflect returning errors.
  [#1467](https://github.com/bytecodealliance/wasmtime/pull/1467)

* Bindings for .NET have moved to
  https://github.com/bytecodealliance/wasmtime-dotnet.
  [#1477](https://github.com/bytecodealliance/wasmtime/pull/1477)

* Passing too many imports to `Instance::new` is now considered an error.
  [#1478](https://github.com/bytecodealliance/wasmtime/pull/1478)

### Fixed

* Spurious segfaults due to out-of-stack conditions when handling signals have
  been fixed.
  [#1315](https://github.com/bytecodealliance/wasmtime/pull/1315)

--------------------------------------------------------------------------------

## 0.15.0

Released 2020-03-31.

### Fixed

Full release produced for all artifacts to account for hiccups in 0.13.0 and
0.14.0.

--------------------------------------------------------------------------------

## 0.14.0

*This version ended up not getting a full release*

### Fixed

Fix build errors in wasi-common on Windows.

--------------------------------------------------------------------------------

## 0.13.0

Released 2020-03-24.

### Added

* Lots of documentation of `wasmtime` has been updated. Be sure to check out the
  [book](https://bytecodealliance.github.io/wasmtime/) and [API
  documentation](https://bytecodealliance.github.io/wasmtime/api/wasmtime/)!

* All wasmtime example programs are now in a top-level `examples` directory and
  are available in both C and Rust.
  [#1286](https://github.com/bytecodealliance/wasmtime/pull/1286)

* A `wasmtime::Linker` type was added to conveniently link link wasm modules
  together and create instances that reference one another.
  [#1384](https://github.com/bytecodealliance/wasmtime/pull/1384)

* Wasmtime now has "jitdump" support enabled by default which allows [profiling
  wasm code on linux][jitdump].
  [#1310](https://github.com/bytecodealliance/wasmtime/pull/1310)

* The `wasmtime::Caller` type now exists as a first-class way to access the
  caller's exports, namely memory, when implementing host APIs. This can be the
  first argument of functions defined with `Func::new` or `Func::wrap` which
  allows easily implementing methods which take a pointer into wasm memory. Note
  that this only works for accessing the caller's `Memory` for now and it must
  be exported. This will eventually be replaced with a more general-purpose
  mechanism like interface types.
  [#1290](https://github.com/bytecodealliance/wasmtime/pull/1290)

* The bulk memory proposal has been fully implemented.
  [#1264](https://github.com/bytecodealliance/wasmtime/pull/1264)
  [#976](https://github.com/bytecodealliance/wasmtime/pull/976)

* Virtual file support has been added to `wasi-common`.
  [#701](https://github.com/bytecodealliance/wasmtime/pull/701)

* The C API has been enhanced with a Wasmtime-specific `wasmtime_wat2wasm` to
  parse `*.wat` files via the C API.
  [#1206](https://github.com/bytecodealliance/wasmtime/pull/1206)

[jitdump]: https://bytecodealliance.github.io/wasmtime/examples-profiling.html

### Changed

* The `wast` and `wasm2obj` standalone binaries have been removed. They're
  available via the `wasmtime wast` and `wasmtime wasm2obj` subcommands.
  [#1372](https://github.com/bytecodealliance/wasmtime/pull/1372)

* The `wasi-common` crate now uses the new `wiggle` crate to auto-generate a
  trait which is implemented for the current wasi snapshot.
  [#1202](https://github.com/bytecodealliance/wasmtime/pull/1202)

* Wasmtime no longer has a dependency on a C++ compiler.
  [#1365](https://github.com/bytecodealliance/wasmtime/pull/1365)

* The `Func::wrapN` APIs have been consolidated into one `Func::wrap` API.
  [#1363](https://github.com/bytecodealliance/wasmtime/pull/1363)

* The `Callable` trait has been removed and now `Func::new` takes a closure
  directly.
  [#1363](https://github.com/bytecodealliance/wasmtime/pull/1363)

* The Cranelift repository has been merged into the Wasmtime repository.

* Support for interface types has been temporarily removed.
  [#1292](https://github.com/bytecodealliance/wasmtime/pull/1292)

* The exit code of the `wasmtime` CLI has changed if the program traps.
  [#1274](https://github.com/bytecodealliance/wasmtime/pull/1274)

* The `wasmtime` CLI now logs to stderr by default and the `-d` flag has been
  renamed to `--log-to-file`.
  [#1266](https://github.com/bytecodealliance/wasmtime/pull/1266)

* Values cannot cross `Store` objects, meaning you can't instantiate a module
  with values from different stores nor pass values from different stores into
  methods.
  [#1016](https://github.com/bytecodealliance/wasmtime/pull/1016)

--------------------------------------------------------------------------------

## 0.12.0

Released 2020-02-26.

### Added

* Support for the [WebAssembly text annotations proposal][annotations-proposal]
  has been added.
  [#998](https://github.com/bytecodealliance/wasmtime/pull/998)

* An initial C API for instantiating WASI modules has been added.
  [#977](https://github.com/bytecodealliance/wasmtime/pull/977)

* A new suite of `Func::getN` functions have been added to the `wasmtime` API to
  call statically-known function signatures in a highly optimized fashion.
  [#955](https://github.com/bytecodealliance/wasmtime/pull/955)

* Initial support for profiling JIT code through perf jitdump has been added.
  [#360](https://github.com/bytecodealliance/wasmtime/pull/360)

* More CLI flags corresponding to proposed WebAssembly features have been added.
  [#917](https://github.com/bytecodealliance/wasmtime/pull/917)

[annotations-proposal]: https://github.com/webassembly/annotations

### Changed

* The `wasmtime` CLI as well as embedding API will optimize WebAssembly code by
  default now.
  [#973](https://github.com/bytecodealliance/wasmtime/pull/973)
  [#988](https://github.com/bytecodealliance/wasmtime/pull/988)

* The `verifier` pass in Cranelift is now no longer run by default when using
  the embedding API.
  [#882](https://github.com/bytecodealliance/wasmtime/pull/882)

### Fixed

* Code caching now accurately accounts for optimization levels, ensuring that if
  you ask for optimized code you're not accidentally handed unoptimized code
  from the cache.
  [#974](https://github.com/bytecodealliance/wasmtime/pull/974)

* Automated releases for tags should be up and running again, along with
  automatic publication of the `wasmtime` Python package.
  [#971](https://github.com/bytecodealliance/wasmtime/pull/971)
