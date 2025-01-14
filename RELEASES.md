## 28.0.1

Released 2025-01-14.

### Fixed

* Fixed deallocating async stacks when using `Store::into_data`.
  [#10009](https://github.com/bytecodealliance/wasmtime/pull/10009)

--------------------------------------------------------------------------------

## 28.0.0

Released 2024-12-20.

### Added

* The ISLE DSL used for Cranelift now has a first-class `bool` type.
  [#9593](https://github.com/bytecodealliance/wasmtime/pull/9593)

* Cranelift now supports a new single-pass register allocator designed for
  compile-time performance (unlike the current default which is optimized for
  runtime-of-generated-code performance).
  [#9611](https://github.com/bytecodealliance/wasmtime/pull/9611)

* The `wasmtime` crate now natively supports the `wasm-wave` crate and its
  encoding of component value types.
  [#8872](https://github.com/bytecodealliance/wasmtime/pull/8872)

* A `Module` can now be created from an already-open file.
  [#9571](https://github.com/bytecodealliance/wasmtime/pull/9571)

* A new default-enabled crate feature, `signals-based-traps`, has been added to
  the `wasmtime` crate. When disabled then runtime signal handling is not
  required by the host. This is intended to help with future effort to port
  Wasmtime to more platforms.
  [#9614](https://github.com/bytecodealliance/wasmtime/pull/9614)

* Linear memories may now be backed by `malloc` in certain conditions when guard
  pages are disabled, for example.
  [#9614](https://github.com/bytecodealliance/wasmtime/pull/9614)
  [#9634](https://github.com/bytecodealliance/wasmtime/pull/9634)

* Wasmtime's `async` feature no longer requires `std`.
  [#9689](https://github.com/bytecodealliance/wasmtime/pull/9689)

* The buffer and budget capacity of `OutgoingBody` in `wasmtime-wasi-http` are
  now configurable.
  [#9670](https://github.com/bytecodealliance/wasmtime/pull/9670)

### Changed

* Wasmtime's external and internal distinction of "static" and "dynamic"
  memories has been refactored and reworded. All options are preserved but
  exported under different names with improved documentation about how they all
  interact with one another. (and everything should be easier to understand)
  [#9545](https://github.com/bytecodealliance/wasmtime/pull/9545)

* Each `Store<T>` now caches a single fiber stack in async mode to avoid
  allocating/deallocating if the store is used multiple times.
  [#9604](https://github.com/bytecodealliance/wasmtime/pull/9604)

* Linear memories now have a 32MiB guard region at the end instead of a 2GiB
  guard region by default.
  [#9606](https://github.com/bytecodealliance/wasmtime/pull/9606)

* Wasmtime will no longer validate dependencies between WebAssembly features,
  instead delegating this work to `wasmparser`'s validator.
  [#9623](https://github.com/bytecodealliance/wasmtime/pull/9623)

* Cranelift's `isle-in-source-tree` feature has been re-worked as an environment
  variable.
  [#9633](https://github.com/bytecodealliance/wasmtime/pull/9633)

* Wasmtime's minimum supported Rust version is now 1.81.
  [#9692](https://github.com/bytecodealliance/wasmtime/pull/9692)

* Synthetic types in DWARF are now more efficiently represented.
  [#9700](https://github.com/bytecodealliance/wasmtime/pull/9700)

* Debug builtins on Windows are now exported correctly.
  [#9706](https://github.com/bytecodealliance/wasmtime/pull/9706)

* Documentation on `Config` now clarifies that defaults of some options may
  differ depending on the selected target or compiler depending on features
  supported.
  [#9705](https://github.com/bytecodealliance/wasmtime/pull/9705)

* Wasmtime's error-related types now all unconditionally implement the `Error`
  trait, even in `#[no_std]` mode.
  [#9702](https://github.com/bytecodealliance/wasmtime/pull/9702)

### Fixed

* Field type matching for subtyping with wasm GC has been fixed.
  [#9724](https://github.com/bytecodealliance/wasmtime/pull/9724)

* Native unwind info generated for s390x has been fixed in the face of tail
  calls.
  [#9725](https://github.com/bytecodealliance/wasmtime/pull/9725)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
