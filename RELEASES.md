## 44.0.0

Unreleased.

### Added

* The `wasmtime` CLI now supports a `-g` flag which runs a built-in wasm program
  to host a `gdbstub`-compatible server (can be connected to with LLDB) to debug
  guest programs.
  [#12756](https://github.com/bytecodealliance/wasmtime/pull/12756)
  [#12771](https://github.com/bytecodealliance/wasmtime/pull/12771)
  [#12856](https://github.com/bytecodealliance/wasmtime/pull/12856)
  [#12859](https://github.com/bytecodealliance/wasmtime/pull/12859)

* Wasmtime now has experimental support for the `map<K, V>` type in the
  component model.
  [#12216](https://github.com/bytecodealliance/wasmtime/pull/12216)

* Wasmtime's C API now supports wasm tag types.
  [#12763](https://github.com/bytecodealliance/wasmtime/pull/12763)
  [#12803](https://github.com/bytecodealliance/wasmtime/pull/12803)

* Wasmtime's C API now supports exceptions.
  [#12861](https://github.com/bytecodealliance/wasmtime/pull/12861)

* Wasmtime's C API has more support for the GC proposal.
  [#12914](https://github.com/bytecodealliance/wasmtime/pull/12914)
  [#12915](https://github.com/bytecodealliance/wasmtime/pull/12915)
  [#12916](https://github.com/bytecodealliance/wasmtime/pull/12916)
  [#12917](https://github.com/bytecodealliance/wasmtime/pull/12917)

* An initial implementation of the `wasi:tls` proposal for the 0.3.0-draft
  version has been added.
  [#12834](https://github.com/bytecodealliance/wasmtime/pull/12834)

### Changed

* The `demangle` Cargo feature of the `wasmtime` crate is now compatible with
  `no_std` targets.
  [#12740](https://github.com/bytecodealliance/wasmtime/pull/12740)

* The `wasmtime-wasi-tls-*` crates are now merged into one crate with feature
  flags for each backend.
  [#12780](https://github.com/bytecodealliance/wasmtime/pull/12780)

* Wasmtime now requires Rust 1.92.0 or later to build.
  [#12828](https://github.com/bytecodealliance/wasmtime/pull/12828)

* The `cranelift-codegen` crate now compiles for `no_std` targets.
  [#12812](https://github.com/bytecodealliance/wasmtime/pull/12812)

* The `csdb` instruction, a defense-in-depth measure for spectre, is no longer
  emitted by default on aarch64 to match what peer runtimes are doing. In some
  situations this is known to provide up to a 6x performance boost on macOS as
  well.
  [#12932](https://github.com/bytecodealliance/wasmtime/pull/12932)

### Fixed

* Wasmtime's native DWARF has been improved on aarch64 to support recovering
  values more frequently.
  [#12779](https://github.com/bytecodealliance/wasmtime/pull/12779)

* A significant number of minor issues have been fixed throughout this release.
  In addition to the [security advisories][adv] found by LLMs there have also
  been a large number of other issues identified as well. Many minor fixes are
  present in this release for various situations for issues found in this
  manner.

[adv]: https://bytecodealliance.org/articles/wasmtime-security-advisories

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
