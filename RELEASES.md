## 32.0.0

Released 2025-04-21.

### Added

* `{Module,Component}::deserialize_raw` can now be used to deserialize an
  in-memory module while relying on external management of the memory.
  [#10321](https://github.com/bytecodealliance/wasmtime/pull/10321)

* An initial implementation of wasi-tls has been added.
  [#10249](https://github.com/bytecodealliance/wasmtime/pull/10249)

* The `wasmtime` CLI now supports hexadecimal integer CLI arguments.
  [#10360](https://github.com/bytecodealliance/wasmtime/pull/10360)

* Cranelift now supports a `log2_min_function_alignment` flag.
  [#10391](https://github.com/bytecodealliance/wasmtime/pull/10391)

* A new `wasmtime objdump` subcommand has been added to help explore and debug
  `*.cwasm` files.
  [#10405](https://github.com/bytecodealliance/wasmtime/pull/10405)

* Support for the pooling allocator has been added to the C API.
  [#10484](https://github.com/bytecodealliance/wasmtime/pull/10484)

* Support for the guest profiler with the component model has been added.
  [#10507](https://github.com/bytecodealliance/wasmtime/pull/10507)

### Changed

* Cranelift `MemFlags` now has a `can_move` flag which restricts whether a load
  or store can be moved.
  [#10340](https://github.com/bytecodealliance/wasmtime/pull/10340)

* The `.text` size of Pulley `*.cwasm` files should be smaller with less
  padding.
  [#10285](https://github.com/bytecodealliance/wasmtime/pull/10285)

* The `wasmtime serve` subcommand now implements a graceful shutdown on ctrl-c.
  [#10394](https://github.com/bytecodealliance/wasmtime/pull/10394)

* Stack maps used for GC are now stored in a serialized binary format that is
  faster to deserialize.
  [#10404](https://github.com/bytecodealliance/wasmtime/pull/10404)

* The aegraph implementation in Cranelift has been simplified to remove the
  union-find and canonical eclass IDs.
  [#10471](https://github.com/bytecodealliance/wasmtime/pull/10471)

* The `store_list` and `load_list` helpers have been specialized in components
  for `f32` and `f64`.
  [#9892](https://github.com/bytecodealliance/wasmtime/pull/9892)

* Cranelift now removes block params on critical-edge blocks.
  [#10485](https://github.com/bytecodealliance/wasmtime/pull/10485)

* The `Linker::define_unknown_imports_as_default_values` API now supports
  defining defaults for more kinds of items.
  [#10500](https://github.com/bytecodealliance/wasmtime/pull/10500)

* Wasmtime now requires Rust 1.84.0 to compile.
  [#10520](https://github.com/bytecodealliance/wasmtime/pull/10520)

### Fixed

* Winch compilation of extadd instructions has been fixed.
  [#10337](https://github.com/bytecodealliance/wasmtime/pull/10337)

* Fix an issue with DRC collector's barriers.
  [#10371](https://github.com/bytecodealliance/wasmtime/pull/10371)

* Loads on `(ref null none)` that can trap are now performed.
  [#10372](https://github.com/bytecodealliance/wasmtime/pull/10372)

* Fix reference count management in `AnyRef::from_raw`.
  [#10374](https://github.com/bytecodealliance/wasmtime/pull/10374)

* An issue with multi-value returns in Winch has been fixed.
  [#10370](https://github.com/bytecodealliance/wasmtime/pull/10370)

* A panic at compile-time from an overflowing shift has been fixed when
  targeting aarch64.
  [#10382](https://github.com/bytecodealliance/wasmtime/pull/10382)

* The `wasmtime serve` command no longer panics when `handle` returns before
  calling `set`.
  [#10387](https://github.com/bytecodealliance/wasmtime/pull/10387)

* Winch compilation of `replace_lane` instructions with floats has been fixed.
  [#10393](https://github.com/bytecodealliance/wasmtime/pull/10393)

* An invalid integer-shift optimization on vector types has been removed.
  [#10413](https://github.com/bytecodealliance/wasmtime/pull/10413)

* The DWARF loclist to exprloc optimization has been fixed.
  [#10400](https://github.com/bytecodealliance/wasmtime/pull/10400)

* Objects in the DRC collector are now transitively dec-ref's when collected.
  [#10401](https://github.com/bytecodealliance/wasmtime/pull/10401)

* A bug with GC rec gropus and registration in an `Engine` has been fixed.
  [#10435](https://github.com/bytecodealliance/wasmtime/pull/10435)

* A bug related to GC arrays of GC refs misreported their count of GC edges has
  been fixed.
  [#10453](https://github.com/bytecodealliance/wasmtime/pull/10453)

* A bug related to appropriately adding stack maps for all GC variables has been
  fixed.
  [#10456](https://github.com/bytecodealliance/wasmtime/pull/10456)
  [#10468](https://github.com/bytecodealliance/wasmtime/pull/10468)

* A bug with `array.fill` has been fixed.
  [#10470](https://github.com/bytecodealliance/wasmtime/pull/10470)

* GC structs are no longer reordered to optimize their size to fix subtyping.
  [#10463](https://github.com/bytecodealliance/wasmtime/pull/10463)

* Panics related to exceptions and components being mixed has been fixed.
  [#10473](https://github.com/bytecodealliance/wasmtime/pull/10473)

* Winch stack parameter alignment has been fixed.
  [#10513](https://github.com/bytecodealliance/wasmtime/pull/10513)

* Rendering inline function frames in a trap backtrace has been fixed.
  [#10523](https://github.com/bytecodealliance/wasmtime/pull/10523)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
