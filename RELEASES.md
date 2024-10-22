## 26.0.0

Released 2024-10-22.

### Added

* The "table64" extension of the memory64 proposals to WebAssembly has been
  implemented.
  [#9206](https://github.com/bytecodealliance/wasmtime/pull/9206)

* Initial support has been added for compiling WebAssembly modules with Pulley,
  Wasmtime's interpreter. Note that the interpreter is not feature complete yet.
  [#9240](https://github.com/bytecodealliance/wasmtime/pull/9240)

* Wasmtime can now execute code without relying on host-based signal handlers.
  [#9230](https://github.com/bytecodealliance/wasmtime/pull/9230)

* Work has continued on implementing the GC proposals in Wasmtime.
  [#9246](https://github.com/bytecodealliance/wasmtime/pull/9246)
  [#9244](https://github.com/bytecodealliance/wasmtime/pull/9244)
  [#9271](https://github.com/bytecodealliance/wasmtime/pull/9271)
  [#9275](https://github.com/bytecodealliance/wasmtime/pull/9275)
  [#9278](https://github.com/bytecodealliance/wasmtime/pull/9278)
  [#9282](https://github.com/bytecodealliance/wasmtime/pull/9282)
  [#9285](https://github.com/bytecodealliance/wasmtime/pull/9285)
  [#9326](https://github.com/bytecodealliance/wasmtime/pull/9326)
  [#9341](https://github.com/bytecodealliance/wasmtime/pull/9341)
  [#9358](https://github.com/bytecodealliance/wasmtime/pull/9358)

* Support for ARM64 Windows has been finished with support for unwinding.
  Release binaries are now also available for this platform.
  [#9266](https://github.com/bytecodealliance/wasmtime/pull/9266)
  [#9283](https://github.com/bytecodealliance/wasmtime/pull/9283)

* The `bindgen!` macro now supports multiple paths to load WIT from.
  [#9288](https://github.com/bytecodealliance/wasmtime/pull/9288)

* A new `-W async-stack-size=N` argument has been added to the CLI.
  [#9302](https://github.com/bytecodealliance/wasmtime/pull/9302)

* A new `wasmtime completion` subcommand can be used to generate a completion
  script for the Wasmtime CLI.
  [#9312](https://github.com/bytecodealliance/wasmtime/pull/9312)

* Components now support `initialize_copy_on_write_image` like core modules.
  [#9357](https://github.com/bytecodealliance/wasmtime/pull/9357)

* Initial support for the ISLE verifier Crocus has landed.
  [#9178](https://github.com/bytecodealliance/wasmtime/pull/9178)

### Changed

* Wasmtime now requires Rust 1.79.0 to compile.
  [#9202](https://github.com/bytecodealliance/wasmtime/pull/9202)

* The `future-trailers.get` in `wasi-http` now returns `none` when trailers are
  absent.
  [#9208](https://github.com/bytecodealliance/wasmtime/pull/9208)

* The Cranelift instructions `iadd_cin` and `isub_bin` were removed. The
  `isub_borrow` and `iadd_carry` instructions were renamed to `{u,s}add_carry`
  and `{u,s}sub_borrow`.
  [#9199](https://github.com/bytecodealliance/wasmtime/pull/9199)

* Winch now supports multi-value results on AArch64.
  [#9218](https://github.com/bytecodealliance/wasmtime/pull/9218)

* Some issues related to `shutdown` have been fixed with WASI sockets.
  [#9225](https://github.com/bytecodealliance/wasmtime/pull/9225)

* Cranelift now has a Cargo feature to enable support for all native ISAs and
  not Pulley.
  [#9237](https://github.com/bytecodealliance/wasmtime/pull/9237)

* Cranelift support for `StructArgument` in the arm64, riscv64, and s390x
  backends has been removed.
  [#9258](https://github.com/bytecodealliance/wasmtime/pull/9258)

* The pooling allocator no longer limits instances/memories/tables by default.
  [#9257](https://github.com/bytecodealliance/wasmtime/pull/9257)

* Stack overflow on an async stack will now print a message that this happened.
  [#9304](https://github.com/bytecodealliance/wasmtime/pull/9304)

* Cranelift's `cranelift-wasm` crate has been removed and folded directly into
  `wasmtime-cranelift`.
  [#9313](https://github.com/bytecodealliance/wasmtime/pull/9313)

* Cranelift's `TrapCode` type is now represented with a single byte.
  [#9338](https://github.com/bytecodealliance/wasmtime/pull/9338)

### Fixed

* Stack slots in Cranelift are now aligned from the start instead of the end.
  [#9279](https://github.com/bytecodealliance/wasmtime/pull/9279)

* The WASIp1 adapter now correctly handles allocations where the initial
  alignment consumes the entire allocation.
  [#9356](https://github.com/bytecodealliance/wasmtime/pull/9356)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
