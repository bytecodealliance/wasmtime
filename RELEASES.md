## 50.0.0

Unreleased.

### Added

* Wasmtime now supports the WebAssembly compact imports proposal, disabled by
  default, via `Config::wasm_compact_imports`.
  [#14335](https://github.com/bytecodealliance/wasmtime/pull/14335)

* The Wasmtime CLI now has options to configure unsafe intrinsics and
  compile-time builtins.
  [#14279](https://github.com/bytecodealliance/wasmtime/pull/14279)

* The Wasmtime CLI and C API can now enable the component model's canonical
  names and accessors features.
  [#14344](https://github.com/bytecodealliance/wasmtime/pull/14344)
  [#14497](https://github.com/bytecodealliance/wasmtime/pull/14497)

* A new `Store::gc_heap_grow` method can be used to manually grow the GC heap
  of a store.
  [#14341](https://github.com/bytecodealliance/wasmtime/pull/14341)

* A new `ResourceAny::resource_drop_concurrent` method can be used to drop a
  guest resource with only an `Accessor`.
  [#14362](https://github.com/bytecodealliance/wasmtime/pull/14362)

* A new `Engine::release_idle_pool_memory` method can be used to release memory
  kept resident by the pooling allocator for idle slots.
  [#14419](https://github.com/bytecodealliance/wasmtime/pull/14419)
  [#14507](https://github.com/bytecodealliance/wasmtime/pull/14507)

* A new off-by-default `task-group-hook` feature enables embedders to associate
  guest-to-host calls with their corresponding host-to-guest calls when using
  component-model-async.
  [#14416](https://github.com/bytecodealliance/wasmtime/pull/14416)

* The `wasmtime-wasi` and `wasmtime-wasi-http` crates now support "named
  imports", the `implements` field of the component model.
  [#14275](https://github.com/bytecodealliance/wasmtime/pull/14275)

* The `wasmtime explore` command now supports components and has resizable
  panes.
  [#14426](https://github.com/bytecodealliance/wasmtime/pull/14426)
  [#14427](https://github.com/bytecodealliance/wasmtime/pull/14427)
  [#14458](https://github.com/bytecodealliance/wasmtime/pull/14458)

* The debugger now exposes shared memories to gdbstub.
  [#14087](https://github.com/bytecodealliance/wasmtime/pull/14087)

* Winch now supports tail calls on x86_64 and AArch64.
  [#14349](https://github.com/bytecodealliance/wasmtime/pull/14349)

* Cranelift has initial support for some x86_64 APX instructions.
  [#13916](https://github.com/bytecodealliance/wasmtime/pull/13916)

* The `wasmtime serve` command can now inherit listening sockets from a service
  manager (e.g. systemd socket units) with a new `--systemd-listenfd` option,
  can serve on Unix sockets, and can automatically exit when idle with a new
  `--idle-process-timeout` option.
  [#14294](https://github.com/bytecodealliance/wasmtime/pull/14294)
  [#14320](https://github.com/bytecodealliance/wasmtime/pull/14320)
  [#14392](https://github.com/bytecodealliance/wasmtime/pull/14392)

### Changed

* Epoch deadline and call-hook callbacks now receive a `StoreHookState`
  instead of a `StoreContextMut`, which only allows access to the `T` in a
  `Store<T>`.
  [#14386](https://github.com/bytecodealliance/wasmtime/pull/14386)

* The `wasmtime` CLI no longer enables WASIp0 (`wasi_unstable`) by default. It
  can be enabled with `-Spreview0`.
  [#14479](https://github.com/bytecodealliance/wasmtime/pull/14479)

* Winch no longer enables the GC types, exceptions, or tail-call proposals by
  default as support for these is not yet Tier 1.
  [#14449](https://github.com/bytecodealliance/wasmtime/pull/14449)
  [#14502](https://github.com/bytecodealliance/wasmtime/pull/14502)

* `Accessor::spawn` now takes an async closure instead of an `AccessorTask`.
  [#14376](https://github.com/bytecodealliance/wasmtime/pull/14376)

* Component-model-async has been updated to the latest specification: the
  `cancellable` immediate has been removed from intrinsics, `subtask.cancel`
  semantics have been refined, `task.cancel` now traps before the cancel event
  is delivered, and use of stream/future handles after the other end has been
  dropped now traps.
  [#14244](https://github.com/bytecodealliance/wasmtime/pull/14244)
  [#14340](https://github.com/bytecodealliance/wasmtime/pull/14340)
  [#14342](https://github.com/bytecodealliance/wasmtime/pull/14342)
  [#14378](https://github.com/bytecodealliance/wasmtime/pull/14378)
  [#14382](https://github.com/bytecodealliance/wasmtime/pull/14382)

* Component-model-async now periodically yields during concurrent work to avoid
  starving other host tasks.
  [#14460](https://github.com/bytecodealliance/wasmtime/pull/14460)

* The performance of async guest-to-host calls in component-model-async has
  been improved by lazily creating host tasks, and sync-to-sync adapters no
  longer need to call `{enter,exit}-sync-call` in many cases.
  [#14350](https://github.com/bytecodealliance/wasmtime/pull/14350)
  [#14270](https://github.com/bytecodealliance/wasmtime/pull/14270)

* Wasmtime now drops cached compiler contexts between compiling
  modules/components, reducing sustained memory usage.
  [#14314](https://github.com/bytecodealliance/wasmtime/pull/14314)

* Cranelift's alias analysis is faster and more precise, and GC struct fields,
  array elements, and GC header fields now all have their own alias regions.
  [#14290](https://github.com/bytecodealliance/wasmtime/pull/14290)
  [#14425](https://github.com/bytecodealliance/wasmtime/pull/14425)
  [#14351](https://github.com/bytecodealliance/wasmtime/pull/14351)
  [#14353](https://github.com/bytecodealliance/wasmtime/pull/14353)

* Wasmtime now uses bulk libunwind section registration when available.
  [#14304](https://github.com/bytecodealliance/wasmtime/pull/14304)

* Fuel is now charged for more host allocations made with `Val`.
  [#14415](https://github.com/bytecodealliance/wasmtime/pull/14415)

* Default `wasi:http` hooks now classify connection and TLS errors in more
  detail.
  [#14501](https://github.com/bytecodealliance/wasmtime/pull/14501)

* `wasi:http` now always accepts an empty path-with-query.
  [#14446](https://github.com/bytecodealliance/wasmtime/pull/14446)

* The CLI now rejects zero for the `--http-outgoing-body-*` options.
  [#14448](https://github.com/bytecodealliance/wasmtime/pull/14448)

* Work continues on implementing the stack-switching proposal.
  [#14140](https://github.com/bytecodealliance/wasmtime/pull/14140)
  [#14369](https://github.com/bytecodealliance/wasmtime/pull/14369)
  [#14398](https://github.com/bytecodealliance/wasmtime/pull/14398)
  [#14441](https://github.com/bytecodealliance/wasmtime/pull/14441)

* Wasmtime now requires Rust 1.97.0 to compile.
  [#14466](https://github.com/bytecodealliance/wasmtime/pull/14466)

* The pooling allocator now scans further than a fixed set of region and
  additionally correctly resets pages that have been paged out.
  [#14357](https://github.com/bytecodealliance/wasmtime/pull/14357)

### Fixed

* Converting a borrowed `Resource<T>` or `ResourceDynamic` created with
  `new_borrow` into a `ResourceAny` outside of a component call no longer
  panics. The borrow is now lowered into the guest when the call happens, and
  `ResourceAny::resource_drop` is no longer required for such borrows.
  [#14384](https://github.com/bytecodealliance/wasmtime/pull/14384)

* Recursive `run_concurrent` is now allowed for disparate stores.
  [#14302](https://github.com/bytecodealliance/wasmtime/pull/14302)

* Fuel is now correctly saved and restored around `call_ref` and exception
  catches in Cranelift, and buffered fuel charges in a module's start function
  are no longer dropped.
  [#14407](https://github.com/bytecodealliance/wasmtime/pull/14407)
  [#14391](https://github.com/bytecodealliance/wasmtime/pull/14391)

* The `check-write` budget is now enforced in `wasi:http` outgoing bodies and
  some `wasi:io` output streams.
  [#14408](https://github.com/bytecodealliance/wasmtime/pull/14408)
  [#14429](https://github.com/bytecodealliance/wasmtime/pull/14429)
  [#14505](https://github.com/bytecodealliance/wasmtime/pull/14505)

* `Store::set_epoch_deadline` no longer overflows with very large deltas.
  [#14513](https://github.com/bytecodealliance/wasmtime/pull/14513)

* JIT code is now made coherent with the instruction cache correctly on
  AArch64.
  [#14454](https://github.com/bytecodealliance/wasmtime/pull/14454)

* Native stack-walking on macOS arm64 has been fixed.
  [#14303](https://github.com/bytecodealliance/wasmtime/pull/14303)

* A stack overflow translating a variant with a huge number of cases between
  components has been fixed.
  [#14355](https://github.com/bytecodealliance/wasmtime/pull/14355)

* Resource aliases are now preserved when stubbing unknown imports.
  [#14396](https://github.com/bytecodealliance/wasmtime/pull/14396)

* `bindgen!` now uses fully-qualified paths for prelude types to avoid
  collisions with WIT type names.
  [#14543](https://github.com/bytecodealliance/wasmtime/pull/14543)

* Validation of `if` without `else` in the presence of subtyping has been fixed.
  [#14334](https://github.com/bytecodealliance/wasmtime/pull/14334)

* A debug assertion in `ref.test` of internalized `externref`s has been fixed.
  [#14315](https://github.com/bytecodealliance/wasmtime/pull/14315)

* Cranelift now allows returns via `xmm1` with `WindowsFastCall` for
  `extern "Rust"`.
  [#14361](https://github.com/bytecodealliance/wasmtime/pull/14361)

* `cranelift-object` now sets symbol flags correctly and returns an error,
  instead of panicking, for data objects declared with different TLS values.
  [#14519](https://github.com/bytecodealliance/wasmtime/pull/14519)
  [#14469](https://github.com/bytecodealliance/wasmtime/pull/14469)

* Winch now handles unreachable `try_table`, emits fuel updates before throwing
  exceptions, and preserves typed `select` result types for GC stack maps.
  [#14379](https://github.com/bytecodealliance/wasmtime/pull/14379)
  [#14297](https://github.com/bytecodealliance/wasmtime/pull/14297)
  [#14455](https://github.com/bytecodealliance/wasmtime/pull/14455)

* `wasmtime run` now honors the `--http-outgoing-body-*` options.
  [#14440](https://github.com/bytecodealliance/wasmtime/pull/14440)

* Graceful shutdown of `wasmtime serve` has been fixed.
  [#14388](https://github.com/bytecodealliance/wasmtime/pull/14388)

* The CLI no longer panics on overflow when deriving the async stack size.
  [#14514](https://github.com/bytecodealliance/wasmtime/pull/14514)

* A number of `wasi:filesystem` fixes: `set-times` on directory descriptors,
  `stat` of symlinks with trailing slashes, the `stat` fast path, datetime
  overflow, and directory entry file types on filesystems without `d_type`.
  [#14420](https://github.com/bytecodealliance/wasmtime/pull/14420)
  [#14387](https://github.com/bytecodealliance/wasmtime/pull/14387)
  [#14368](https://github.com/bytecodealliance/wasmtime/pull/14368)
  [#14403](https://github.com/bytecodealliance/wasmtime/pull/14403)
  [#14381](https://github.com/bytecodealliance/wasmtime/pull/14381)

* Accepted TCP sockets now preserve their peer address after a reset.
  [#14443](https://github.com/bytecodealliance/wasmtime/pull/14443)

* `MACH_RCV_INTERRUPT` is no longer used in the macOS mach ports
  implementation.
  [#14414](https://github.com/bytecodealliance/wasmtime/pull/14414)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
* [49.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-49.0.0/RELEASES.md)
* [48.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-48.0.0/RELEASES.md)
* [47.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-47.0.0/RELEASES.md)
* [46.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-46.0.0/RELEASES.md)
* [45.0.x](https://github.com/bytecodealliance/wasmtime/blob/release-45.0.0/RELEASES.md)
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
