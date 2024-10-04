## 22.0.1

Released 2024-10-09.

### Fixed

* Fix a runtime crash when combining tail-calls with host imports that capture a
  stack trace or trap.
  [GHSA-q8hx-mm92-4wvg](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-q8hx-mm92-4wvg)

* Fix a race condition could lead to WebAssembly control-flow integrity and type
  safety violations.
  [GHSA-7qmx-3fpx-r45m](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-7qmx-3fpx-r45m)

--------------------------------------------------------------------------------

## 22.0.0

Released 2024-06-20.

### Added

* The CMake-based build of Wasmtime's C API now adds a
  `WASMTIME_FASTEST_RUNTIME` option which activates the "fastest-runtime"
  compilation profile which enables LTO.
  [#8554](https://github.com/bytecodealliance/wasmtime/pull/8554)

* Cranelift supports more instructions in the RISC-V Zfa and ZiCond extensions.
  [#8582](https://github.com/bytecodealliance/wasmtime/pull/8582)
  [#8695](https://github.com/bytecodealliance/wasmtime/pull/8695)

* Support for fused-multiply-and-add on RISC-V has been improved.
  [#8596](https://github.com/bytecodealliance/wasmtime/pull/8596)
  [#8588](https://github.com/bytecodealliance/wasmtime/pull/8588)

* Release binaries for `{aarch64,x86_64}-android` have been added. Note that
  Android is still a [Tier 3 target][target].
  [#8601](https://github.com/bytecodealliance/wasmtime/pull/8601)

* Wasmtime now supports supertypes and finality in the type hierarchy for wasm
  gc.
  [#8595](https://github.com/bytecodealliance/wasmtime/pull/8595)

* Lazy initialization of tables can now be tuned with a CLI flags and
  configuration option.
  [#8531](https://github.com/bytecodealliance/wasmtime/pull/8531)

* Wasmtime now compiles for x86\_64 OpenBSD. Note that this is a [Tier 3
  target][target] and continued support is always appreciated.
  [#8613](https://github.com/bytecodealliance/wasmtime/pull/8613)

* Stack slots in Cranelift can now specify custom alignment.
  [#8635](https://github.com/bytecodealliance/wasmtime/pull/8635)

* Wasm function names are now used in compiled objects to assist with debugging
  and introspection with native tools.
  [#8627](https://github.com/bytecodealliance/wasmtime/pull/8627)

* Wasmtime's release artifacts now includes `wasmtime-platform.h` for use with
  `no_std` targets.
  [#8644](https://github.com/bytecodealliance/wasmtime/pull/8644)

* Release binaries for x86\_64 Alpine Linux have been added.
  [#8668](https://github.com/bytecodealliance/wasmtime/pull/8668)

* A new `Component::define_unknown_imports_as_traps` function has been added to
  stub out functions in a component linker.
  [#8672](https://github.com/bytecodealliance/wasmtime/pull/8672)

[target]: https://docs.wasmtime.dev/stability-tiers.html

### Changed

* Wasmtime and Cranelift's now require Rust 1.76.0 to build.
  [#8560](https://github.com/bytecodealliance/wasmtime/pull/8560)

* The `wasi_config_preopen_dir` function no longer always returns `true` in the
  C API. Additionally `wasi_config_set_env` and `wasi_config_set_argv` may now
  return an error.
  [#8572](https://github.com/bytecodealliance/wasmtime/pull/8572)

* Cranelift now updates registers of backend instructions in-place, simplifying
  register allocation and assignment.
  [#8566](https://github.com/bytecodealliance/wasmtime/pull/8566)
  [#8581](https://github.com/bytecodealliance/wasmtime/pull/8581)
  [#8592](https://github.com/bytecodealliance/wasmtime/pull/8592)
  [#8604](https://github.com/bytecodealliance/wasmtime/pull/8604)
  [#8605](https://github.com/bytecodealliance/wasmtime/pull/8605)

* Wasmtime now attempts to batch memory decommits into one tight loop.
  [#8581](https://github.com/bytecodealliance/wasmtime/pull/8581)
  [#8590](https://github.com/bytecodealliance/wasmtime/pull/8590)

* Bindings generated with `bindgen!` now have generated `GetHost` traits and
  `add_to_linker_get_host` functions which enable a more general means by which
  to acquire host implementations from a store's `T`.
  [#8448](https://github.com/bytecodealliance/wasmtime/pull/8448)

* The `wasmtime serve` subcommand will now dynamically determine whether to use
  the pooling allocator by default based on the system's available virtual
  memory.
  [#8610](https://github.com/bytecodealliance/wasmtime/pull/8610)

* Implementations of `Host` traits in the `wasmtime-wasi` crate are now for
  `WasiImpl<T>` instead of blanket impls for `T`.
  [#8609](https://github.com/bytecodealliance/wasmtime/pull/8609)
  [#8766](https://github.com/bytecodealliance/wasmtime/pull/8766)

* The concepts of "virtual sp offset" and "nominal sp" have been removed from all
  Cranelift backends.
  [#8631](https://github.com/bytecodealliance/wasmtime/pull/8631)
  [#8643](https://github.com/bytecodealliance/wasmtime/pull/8643)

* The maximum size of linear memory in the pooling allocator is now specified in
  bytes instead of pages.
  [#8628](https://github.com/bytecodealliance/wasmtime/pull/8628)

* Wasmtime no longer has two different host ABIs for host functions and instead
  only has one. The "array" calling convention is now unconditionally used
  instead of having a split between the "native" calling convention and the
  "array" calling convention. This means that `Func::new` is now available even
  when the `cranelift` feature is disabled.
  [#8629](https://github.com/bytecodealliance/wasmtime/pull/8629)
  [#8646](https://github.com/bytecodealliance/wasmtime/pull/8646)

* Wasmtime's C API bindings for CMake have been refactored and now supports
  specifying Cargo features directly. Functions that are configured out are now
  also gated in header files.
  [#8642](https://github.com/bytecodealliance/wasmtime/pull/8642)

* Wasmtime's C API can now be built without Cranelift or Winch.
  [#8661](https://github.com/bytecodealliance/wasmtime/pull/8661)

* Wasmtime's release binaries have Winch compiled in by default now.
  [#8660](https://github.com/bytecodealliance/wasmtime/pull/8660)

* The output of `wasmtime explore` now shows function names in addition to
  indices.
  [#8639](https://github.com/bytecodealliance/wasmtime/pull/8639)

* Support for the Wasmtime 13-and-prior CLI has been removed.
  [#8597](https://github.com/bytecodealliance/wasmtime/pull/8597)

* Wiggle-based borrow checking has been removed in favor of modeling host usage
  of guest memory with Rust-level borrows.
  [#8702](https://github.com/bytecodealliance/wasmtime/pull/8702)

* Wasmtime's `bindgen!` macro will now generate the same hierarchy of
  traits/types/modules even when the `with` module is used via new `pub use`
  statements.
  [#8721](https://github.com/bytecodealliance/wasmtime/pull/8721)

* The `WasiCtxBuilder::socket_addr_check` function now takes an `async` closure.
  [#8715](https://github.com/bytecodealliance/wasmtime/pull/8715)

* The `Func::wrapN_async` functions and friends have all been consolidated into
  a single function with a slightly different signature of taking a tuple of
  arguments rather than "splatted" arguments.
  [#8732](https://github.com/bytecodealliance/wasmtime/pull/8732)

### Fixed

* Trampoline lookup for wasm gc functions that may use subtyping on the host to
  match a guest's desired type now no longer panics.
  [#8579](https://github.com/bytecodealliance/wasmtime/pull/8579)

* The total size of arguments, environment variables, and preopens is now
  allowed to exceed 64k when using the wasip1 component adapter.
  [#8594](https://github.com/bytecodealliance/wasmtime/pull/8594)

* Performing a zero-length `read` on file streams is now fixed in WASI.
  [#8611](https://github.com/bytecodealliance/wasmtime/pull/8611)

* Tail calls are now turned by default after a mistake was discovered in the
  previous releases's intent to enable them by default.
  [#8682](https://github.com/bytecodealliance/wasmtime/pull/8682)

* Winch support for `f64` comparison instructions has been fixed.
  [#8685](https://github.com/bytecodealliance/wasmtime/pull/8685)

* The `SO_REUSEADDR` option is reenabled for Unix platforms with `wasmtime
  serve`.
  [#8738](https://github.com/bytecodealliance/wasmtime/pull/8738)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
