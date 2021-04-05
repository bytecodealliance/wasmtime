# Wasmtime Releases

--------------------------------------------------------------------------------

## Unreleased

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

The Rust API does not require a store provided during `Module::new` operation. The `Module` can be send accross threads and instantiate for a specific store. The `Instance::new` now requires the store.

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
