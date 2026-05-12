## 45.0.0

Unreleased.

### Added

* Winch now respects the `enable_nan_canonicalization` setting.
  [#12939](https://github.com/bytecodealliance/wasmtime/pull/12939)

* Initial support for invoking component functions asynchronously has been added
  to the C API.
  [#12973](https://github.com/bytecodealliance/wasmtime/pull/12973)

* Cranelift's s390x backend implements more instructions from z17 and also
  implements more CLIF arithmetic overflow instructions.
  [#12523](https://github.com/bytecodealliance/wasmtime/pull/12523)
  [#12707](https://github.com/bytecodealliance/wasmtime/pull/12707)

* Wasmtime's support for handling OOM in more APIs has expanded and is now
  documented as well.
  [#12993](https://github.com/bytecodealliance/wasmtime/pull/12993)
  [#12988](https://github.com/bytecodealliance/wasmtime/pull/12988)
  [#13017](https://github.com/bytecodealliance/wasmtime/pull/13017)
  [#13047](https://github.com/bytecodealliance/wasmtime/pull/13047)
  [#13049](https://github.com/bytecodealliance/wasmtime/pull/13049)
  [#13051](https://github.com/bytecodealliance/wasmtime/pull/13051)
  [#13074](https://github.com/bytecodealliance/wasmtime/pull/13074)
  [#13083](https://github.com/bytecodealliance/wasmtime/pull/13083)
  [#13088](https://github.com/bytecodealliance/wasmtime/pull/13088)
  [#13224](https://github.com/bytecodealliance/wasmtime/pull/13224)

* The `Component` type now offers reflection APIs over the compiled in-memory
  view of instructions in the same manner `Module` does.
  [#13073](https://github.com/bytecodealliance/wasmtime/pull/13073)

* The `wasmtime` CLI now has a `hot-blocks` subcommand to explore a
  `perf`-recorded output and show hot basic blocks of WebAssembly instructions.
  [#13077](https://github.com/bytecodealliance/wasmtime/pull/13077)

* Wasmtime now has an initial implementation of a copying collector for GC,
  which notably enables collecting cycles unlike the DRC collector.
  [#13093](https://github.com/bytecodealliance/wasmtime/pull/13093)
  [#13107](https://github.com/bytecodealliance/wasmtime/pull/13107)

* The WASI `inherit_network` and `allow_ip_name_lookup` options were added to
  the C API.
  [#13145](https://github.com/bytecodealliance/wasmtime/pull/13145)

* The C API now has the ability to select `Winch` as well as the
  `RegallocAlgorithm` in use.
  [#13155](https://github.com/bytecodealliance/wasmtime/pull/13155)

* The Wasmtime CLI now has a `-Dmax-backtrace=N` argument to control the number
  of frames captured.
  [#13218](https://github.com/bytecodealliance/wasmtime/pull/13218)

* Wasmtime now tracks whether there are any active async tasks within a store
  and provides an embedder API to learn when there are none left.
  [#13246](https://github.com/bytecodealliance/wasmtime/pull/13246)

* Cranelift now has an idempotent-store elimination pass.
  [#13251](https://github.com/bytecodealliance/wasmtime/pull/13251)

### Changed

* Wasmtime's DRC collector has received some optimizations to get some wins on
  local benchmarking.
  [#12969](https://github.com/bytecodealliance/wasmtime/pull/12969)
  [#12974](https://github.com/bytecodealliance/wasmtime/pull/12974)

* Wasmtime's C API can now be built without the `gc` crate feature of Wasmtime.
  [#12805](https://github.com/bytecodealliance/wasmtime/pull/12805)

* Wasmtime now has an implemented and improved grow-vs-collect heuristic to
  improve behavior of GC-using programs.
  [#12942](https://github.com/bytecodealliance/wasmtime/pull/12942)

* Cranelift on aarch64 now uses a more optimized frame layout for tail-call-only
  functions.
  [#11608](https://github.com/bytecodealliance/wasmtime/pull/11608)

* Wasmtime now supports a separate set of GC tunables different from the main
  set of tunables for linear memory to enable configuring it separately.
  [#13080](https://github.com/bytecodealliance/wasmtime/pull/13080)

* Wasmtime no longer uses pointer authentication instructions for the
  implementation of fibers due to issues on Android.
  [#13118](https://github.com/bytecodealliance/wasmtime/pull/13118)

* Wasmtime now requires Rust 1.93.0 to compile.
  [#13127](https://github.com/bytecodealliance/wasmtime/pull/13127)

* The behavior of using Wasmtime as a CMake subproject has been improved.
  [#13157](https://github.com/bytecodealliance/wasmtime/pull/13157)

* Reference types in the C/C++ API have been refactored and reorganized.
  [#13154](https://github.com/bytecodealliance/wasmtime/pull/13154)
  [#13235](https://github.com/bytecodealliance/wasmtime/pull/13235)

* The Wasmtime CLI now warns about usage of wasi-common or wasi-threads as these
  components are slated for removal in Wasmtime 47.0.0. For more information see
  [the associated RFC].
  [#13264](https://github.com/bytecodealliance/wasmtime/pull/13264)

[the associated RFC]: https://github.com/bytecodealliance/rfcs/pull/47

* Inlining in Wasmtime now has a different set of configuration options, notably
  more values are packed into `-Cinlining=...`.
  [#13250](https://github.com/bytecodealliance/wasmtime/pull/13250)

### Fixed

* The WASIp1-to-WASIp2 adapter now handles nonblocking I/O in `fd_{read,write}`
  more appropriately.
  [#13111](https://github.com/bytecodealliance/wasmtime/pull/13111)

* The `Host` header is injected less often for wasmtime-wasi-http.
  [#13138](https://github.com/bytecodealliance/wasmtime/pull/13138)

* Downcasts of `funcref` values now uses the correct type for imported
  functions.
  [#13161](https://github.com/bytecodealliance/wasmtime/pull/13161)

* The DRC allocator's memory usage during tracing has been reduced when there
  are large arrays.
  [#13192](https://github.com/bytecodealliance/wasmtime/pull/13192)

* The performance of reading stdin in WASI has been improved.
  [#13256](https://github.com/bytecodealliance/wasmtime/pull/13256)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
