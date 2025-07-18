## 35.0.0

Unreleased.

### Added

* A new `InputFile` type has been added for specifying stdin as a file in WASI.
  [#10968](https://github.com/bytecodealliance/wasmtime/pull/10968)

* Conditional branches to unconditional traps are now translated to conditional
  traps during legalization.
  [#10988](https://github.com/bytecodealliance/wasmtime/pull/10988)

* The `TE` HTTP header can now be specified by guests.
  [#11002](https://github.com/bytecodealliance/wasmtime/pull/11002)

* Winch on AArch64 should now pass all WebAssembly MVP tests. Note that it is
  still not yet Tier 1 at this time, however.
  [#10829](https://github.com/bytecodealliance/wasmtime/pull/10829)
  [#11013](https://github.com/bytecodealliance/wasmtime/pull/11013)
  [#11031](https://github.com/bytecodealliance/wasmtime/pull/11031)
  [#11051](https://github.com/bytecodealliance/wasmtime/pull/11051)

* The x64 backend now has lowering rules for `{add,sub,or,and} mem, imm`
  [#11043](https://github.com/bytecodealliance/wasmtime/pull/11043)

* Initial support for WASIp2 in the C API has started to land.
  [#11055](https://github.com/bytecodealliance/wasmtime/pull/11055)
  [#11172](https://github.com/bytecodealliance/wasmtime/pull/11172)

* Initial support for GC support in the component model has started to land
  (note that it is not finished yet).
  [#10967](https://github.com/bytecodealliance/wasmtime/pull/10967)
  [#11020](https://github.com/bytecodealliance/wasmtime/pull/11020)

* The `wasmtime-wasi-nn` crate now has a feature to use a custom ONNX runtime.
  [#11060](https://github.com/bytecodealliance/wasmtime/pull/11060)

* Cranelift now optimizes division-by-constant operations to no longer use
  division.
  [#11129](https://github.com/bytecodealliance/wasmtime/pull/11129)

* A `native-tls` backend has been added for the wasi-tls implementation.
  [#11064](https://github.com/bytecodealliance/wasmtime/pull/11064)

### Changed

* Many more instructions for the x64 backend in Cranelift were migrated to the
  new assembler.
  [#10927](https://github.com/bytecodealliance/wasmtime/pull/10927)
  [#10928](https://github.com/bytecodealliance/wasmtime/pull/10928)
  [#10918](https://github.com/bytecodealliance/wasmtime/pull/10918)
  [#10946](https://github.com/bytecodealliance/wasmtime/pull/10946)
  [#10954](https://github.com/bytecodealliance/wasmtime/pull/10954)
  [#10958](https://github.com/bytecodealliance/wasmtime/pull/10958)
  [#10971](https://github.com/bytecodealliance/wasmtime/pull/10971)
  [#10942](https://github.com/bytecodealliance/wasmtime/pull/10942)
  [#10975](https://github.com/bytecodealliance/wasmtime/pull/10975)
  [#11017](https://github.com/bytecodealliance/wasmtime/pull/11017)
  [#10898](https://github.com/bytecodealliance/wasmtime/pull/10898)
  [#10836](https://github.com/bytecodealliance/wasmtime/pull/10836)
  ... (and more)

* Wasmtime internally uses `Pin` for VM data structures to make the internal
  implementations more sound to use. This has no effect on the public API of
  Wasmtime.
  [#10934](https://github.com/bytecodealliance/wasmtime/pull/10934)
  [#10937](https://github.com/bytecodealliance/wasmtime/pull/10937)
  [#10943](https://github.com/bytecodealliance/wasmtime/pull/10943)
  [#10959](https://github.com/bytecodealliance/wasmtime/pull/10959)
  [#11042](https://github.com/bytecodealliance/wasmtime/pull/11042)

* Fused adapters between components now transfer the `enum` component model type
  more efficiently.
  [#10939](https://github.com/bytecodealliance/wasmtime/pull/10939)

* Filenames of `--emit-clif` now match the symbol names found in `*.cwasm`
  artifacts and include the function name as well.
  [#10947](https://github.com/bytecodealliance/wasmtime/pull/10947)
  [#11040](https://github.com/bytecodealliance/wasmtime/pull/11040)

* Wasmtime-internal crates are now all named `wasmtime-internal-*` to even
  further discourage their use.
  [#10963](https://github.com/bytecodealliance/wasmtime/pull/10963)

* Codegen of conditional traps with float compares has been improved.
  [#10966](https://github.com/bytecodealliance/wasmtime/pull/10966)

* More patterns are now optimized in ISLE mid-end rules.
  [#10978](https://github.com/bytecodealliance/wasmtime/pull/10978)
  [#10979](https://github.com/bytecodealliance/wasmtime/pull/10979)
  [#11173](https://github.com/bytecodealliance/wasmtime/pull/11173)

* Winch's support for constants/scratch registers has been improved internally.
  [#10986](https://github.com/bytecodealliance/wasmtime/pull/10986)
  [#10998](https://github.com/bytecodealliance/wasmtime/pull/10998)

* The C API artifacts on Windows are now produced with Clang instead of
  `cl.exe`.
  [#10890](https://github.com/bytecodealliance/wasmtime/pull/10890)

* WebAssembly operand types are now taken into account during translation to
  optimize codegen better in the face of subtyping.
  [#11030](https://github.com/bytecodealliance/wasmtime/pull/11030)

* The behavior of `blocking-write-and-flush` has been updated during flushing
  when `closed` is found.
  [#11018](https://github.com/bytecodealliance/wasmtime/pull/11018)

* WASI WITs have been updated to 0.2.6.
  [#11049](https://github.com/bytecodealliance/wasmtime/pull/11049)

* OpenVINO has been updated to v2025.1.
  [#11054](https://github.com/bytecodealliance/wasmtime/pull/11054)

* The size of the `wasmtime.addrmap` section in `*.cwasm` artifacts has been
  shrunk slightly.
  [#11126](https://github.com/bytecodealliance/wasmtime/pull/11126)

* Authorities in `wasmtime-wasi-http` can now contain the `:` character.
  [#11145](https://github.com/bytecodealliance/wasmtime/pull/11145)

* Wasmtime now requires Rust 1.86 to compile.
  [#11142](https://github.com/bytecodealliance/wasmtime/pull/11142)

* Wasmtime's DRC collector has been optimized and has a new more efficient means
  of managing the set of over-approximated roots on the stack.
  [#11144](https://github.com/bytecodealliance/wasmtime/pull/11144)
  [#11148](https://github.com/bytecodealliance/wasmtime/pull/11148)
  [#11167](https://github.com/bytecodealliance/wasmtime/pull/11167)
  [#11168](https://github.com/bytecodealliance/wasmtime/pull/11168)
  [#11169](https://github.com/bytecodealliance/wasmtime/pull/11169)
  [#11175](https://github.com/bytecodealliance/wasmtime/pull/11175)

* The `ComponentType` trait in Wasmtime now requires the `Send` and `Sync`
  bounds for all implementors.
  [#11160](https://github.com/bytecodealliance/wasmtime/pull/11160)

* The `V128` type is now usable on platforms other than aarch64 and x86\_64.
  [#11165](https://github.com/bytecodealliance/wasmtime/pull/11165)

* Wasmtime's policy on `unsafe` code and guidelines has been added.
  [#11177](https://github.com/bytecodealliance/wasmtime/pull/11177)

* The `std` crate will no longer implicitly be used on `cfg(unix)` and
  `cfg(windows)` targets when the `std` Cargo feature is disabled. This means
  that these platforms now require `std` to be enabled to use the
  platform-specific implementation of linear memory, for example.
  [#11152](https://github.com/bytecodealliance/wasmtime/pull/11152)

### Fixed

* A panic when optimizing `icmp` with vectors has been fixed.
  [#10948](https://github.com/bytecodealliance/wasmtime/pull/10948)

* A panic when lowering `scalar_to_vector` with `i16x8` types has been fixed.
  [#10949](https://github.com/bytecodealliance/wasmtime/pull/10949)

* The vector state register is now considered clobbered by calls on riscv64 to
  ensure it's updated across calls.
  [#11048](https://github.com/bytecodealliance/wasmtime/pull/11048)

* An instance of `gdb` crashing on DWARF emitted by Wasmtime has been fixed.
  [#11077](https://github.com/bytecodealliance/wasmtime/pull/11077)

* Fix a panic in the host caused by preview1 guests using `fd_renumber`.
  [CVE-2025-53901](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-fm79-3f68-h2fc).

* Fix a panic in the preview1 adapter caused by guests using `fd_renumber`.
  [#11277](https://github.com/bytecodealliance/wasmtime/pull/11277)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
