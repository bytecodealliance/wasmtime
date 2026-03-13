## 43.0.0

Unreleased.

### Added

* Wasmtime now supports the WASIp3 snapshot `0.3.0-rc-2026-02-09`.
  [#12557](https://github.com/bytecodealliance/wasmtime/pull/12557)

* The number of frames captured in backtrace collection can now be configured.
  [#12542](https://github.com/bytecodealliance/wasmtime/pull/12542)

* Wasmtime now supports fine-grained operator cost configuration for when fuel
  is enabled.
  [#12541](https://github.com/bytecodealliance/wasmtime/pull/12541)

* Configuring the `gc_support` option is now possible through the C API.
  [#12630](https://github.com/bytecodealliance/wasmtime/pull/12630)

* Configuring the `concurrency_support` option is now possible through the C
  API.
  [#12703](https://github.com/bytecodealliance/wasmtime/pull/12703)

* Debugging-related APIs have been added to access all modules and instances on
  a store.
  [#12637](https://github.com/bytecodealliance/wasmtime/pull/12637)

* All store entities now expose a "unique ID" for debugging purposes.
  [#12645](https://github.com/bytecodealliance/wasmtime/pull/12645)

* Cranelift's x64 backend now supports the `cls` instruction for all integer
  types.
  [#12644](https://github.com/bytecodealliance/wasmtime/pull/12644)

### Changed

* Internal refactoring and support necessary for handling OOM gracefully
  throughout the runtime is proceeding apace. New APIs such as
  `FuncType::try_new` are available in addition to many internal changes.
  [#12530](https://github.com/bytecodealliance/wasmtime/pull/12530)
  [#12537](https://github.com/bytecodealliance/wasmtime/pull/12537)
  (... and many more ...)

* Wasmtime's representation of stack frames in the debugging API no longer
  borrows the store itself and is instead represented as a handle.
  [#12566](https://github.com/bytecodealliance/wasmtime/pull/12566)

* Wasmtime now unconditionally sets `SO_REUSEADDR` for guest-bound sockets.
  [#12597](https://github.com/bytecodealliance/wasmtime/pull/12597)

* Cranelift now supports more `VReg`s which means effectively that larger
  functions will be compilable by default rather than returning a "function too
  large" error.
  [#12611](https://github.com/bytecodealliance/wasmtime/pull/12611)

* WASIp3 implementations now limit returned memory by default for randomness and
  HTTP headers.
  [#12745](https://github.com/bytecodealliance/wasmtime/pull/12745)
  [#12761](https://github.com/bytecodealliance/wasmtime/pull/12761)

### Fixed

* Running `wasmtime wizer` over components with modules that contain an
  `_initialize` function no longer removes the function to preserve the validity
  of the component.
  [#12540](https://github.com/bytecodealliance/wasmtime/pull/12540)

* Borrow state for host async tasks is now handled more correctly throughout
  Wasmtime, especially in the face of cancellation.
  [#12550](https://github.com/bytecodealliance/wasmtime/pull/12550)

* Bindings generation now accounts for the fact that `future` and `stream` are
  not cloneable types.
  [#12155](https://github.com/bytecodealliance/wasmtime/pull/12155)

* The impementation of UDP in WASIp2 has had some wakeup-related bugs fixed.
  [#12629](https://github.com/bytecodealliance/wasmtime/pull/12629)

* Cancellation of host subtasks for component-model-async has been improved and
  works more reliably.
  [#12640](https://github.com/bytecodealliance/wasmtime/pull/12640)

* Subtask management for component-model-async now no longer reparents which
  fixes a number spec-related divergences.
  [#12570](https://github.com/bytecodealliance/wasmtime/pull/12570)

* Converting a `wasmtime::Error` into `anyhow::Error` and using `downcast` has
  been fixed.
  [#12689](https://github.com/bytecodealliance/wasmtime/pull/12689)

* Async stream/future read/write cancellation has had some corner cases fixed.
  [#12704](https://github.com/bytecodealliance/wasmtime/pull/12704)

* Cranelift's timing infrastructure is now more robust in the face of buggy
  system clocks.
  [#12709](https://github.com/bytecodealliance/wasmtime/pull/12709)

* The currently running guest task has been corrected in a number of cases
  related to component-model-async and cooperative threading.
  [#12718](https://github.com/bytecodealliance/wasmtime/pull/12718)
  [#12735](https://github.com/bytecodealliance/wasmtime/pull/12735)
  [#12736](https://github.com/bytecodealliance/wasmtime/pull/12736)
  [#12737](https://github.com/bytecodealliance/wasmtime/pull/12737)

* An issue of lost wakeups with the WASIp3 stdin implementation has been fixed.
  [#12745](https://github.com/bytecodealliance/wasmtime/pull/12745)

--------------------------------------------------------------------------------

Release notes for previous releases of Wasmtime can be found on the respective
release branches of the Wasmtime repository.

<!-- ARCHIVE_START -->
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
