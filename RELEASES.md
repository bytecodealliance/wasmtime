--------------------------------------------------------------------------------

## 0.39.0

Unreleased.

### Added

### Changed

* Some methods on the `Config` structure now return `&mut Self` instead of
  `Result<&mut Self>` since the validation is deferred until `Engine::new`:
  `profiler`, `cranelift_flag_enable`, `cranelift_flag_set`, `max_wasm_stack`,
  `async_stack_size`, and `strategy`.
  [#4252](https://github.com/bytecodealliance/wasmtime/pull/4252)
  [#4262](https://github.com/bytecodealliance/wasmtime/pull/4262)

### Fixed

* A refactor of `Config` was made to fix an issue that the order of calls to `Config`
  matters now, which may lead to unexpected behavior.
  [#4252](https://github.com/bytecodealliance/wasmtime/pull/4252)
  [#4262](https://github.com/bytecodealliance/wasmtime/pull/4262)

--------------------------------------------------------------------------------

## 0.38.1

Released 2022-06-27.

### Fixed.

* A register allocator bug was fixed that could affect direct users of
  Cranelift who use struct-return (`sret`) arguments. The bug had to do with
  the handling of physical register constraints in the function prologue. No
  impact should be possible for users of Cranelift via the Wasm frontend,
  including Wasmtime.
  [regalloc2#60](https://github.com/bytecodealliance/regalloc2/pull/60)

--------------------------------------------------------------------------------

## 0.38.0

Released 2022-06-21

### Added

* Enabling or disabling NaN canonicalization in generated code is now exposed
  through the C API.
  [#4154](https://github.com/bytecodealliance/wasmtime/pull/4154)

* A user-defined callback can now be invoked when an epoch interruption happens
  via the `Store::epoch_deadline_callback` API.
  [#4152](https://github.com/bytecodealliance/wasmtime/pull/4152)

* Basic alias analysis with redundant-load elimintation and store-to-load
  forwarding optimizations has been added to Cranelift.
  [#4163](https://github.com/bytecodealliance/wasmtime/pull/4163)

### Changed

* Traps originating from epoch-based interruption are now exposed as
  `TrapCode::Interrupt`.
  [#4105](https://github.com/bytecodealliance/wasmtime/pull/4105)

* Binary builds for AArch64 now require glibc 2.17 and for s390x require glibc
  2.16. Previously glibc 2.28 was required.
  [#4171](https://github.com/bytecodealliance/wasmtime/pull/4171)

* The `wasmtime::ValRaw` now has all of its fields listed as private and instead
  constructors/accessors are provided for getting at the internal data.
  [#4186](https://github.com/bytecodealliance/wasmtime/pull/4186)

* The `wasm-backtrace` Cargo feature has been removed in favor of a
  `Config::wasm_backtrace` runtime configuration option. Additionally backtraces
  are now only captured when an embedder-generated trap actually reaches a
  WebAssembly call stack.
  [#4183](https://github.com/bytecodealliance/wasmtime/pull/4183)

* Usage of `*_unchecked` APIs for `Func` in the `wasmtime` crate and C API now
  take a `usize` parameter indicating the number of `ValRaw` values behind
  the associated pointer.
  [#4192](https://github.com/bytecodealliance/wasmtime/pull/4192)

### Fixed

* An improvement was made to the spill-slot allocation in code generation to fix
  an issue where some stack slots accidentally weren't reused. This issue was
  introduced with the landing of regalloc2 in 0.37.0 and may have resulted in
  larger-than-intended increases in stack frame sizes.
  [#4222](https://github.com/bytecodealliance/wasmtime/pull/4222)

--------------------------------------------------------------------------------

## 0.37.0

Released 2022-05-20

### Added

* Updated Cranelift to use regalloc2, a new register allocator. This should
  result in ~20% faster compile times, and for programs that suffered from
  register-allocation pressure before, up to ~20% faster generated code.
  [#3989](https://github.com/bytecodealliance/wasmtime/pull/3989)

* Pre-built binaries for macOS M1 machines are now available as release
  artifacts.
  [#3983](https://github.com/bytecodealliance/wasmtime/pull/3983)

* Copy-on-write images of memory can now be manually initialized for a `Module`
  with an explicit method call, but it is still not required to call this method
  and will automatically otherwise happen on the first instantiation.
  [#3964](https://github.com/bytecodealliance/wasmtime/pull/3964)

### Fixed

* Using `InstancePre::instantiate` or `Linker::instantiate` will now panic as
  intended when used with an async-configured `Store`.
  [#3972](https://github.com/bytecodealliance/wasmtime/pull/3972)

### Changed

* The unsafe `ValRaw` type in the `wasmtime` crate now always stores its values
  in little-endian format instead of the prior native-endian format. Users of
  `ValRaw` are recommended to audit their existing code for usage to continue
  working on big-endian platforms.
  [#4035](https://github.com/bytecodealliance/wasmtime/pull/4035)

### Removed

* Support for `Config::paged_memory_initialization` and the `uffd` crate feature
  have been removed from the `wasmtime` crate. Users should migrate to using
  `Config::memory_init_cow` which is more portable and faster at this point.
  [#4040](https://github.com/bytecodealliance/wasmtime/pull/4040)

--------------------------------------------------------------------------------

## 0.36.0

Released 2022-04-20

### Added

* Support for epoch-based interruption has been added to the C API.
  [#3925](https://github.com/bytecodealliance/wasmtime/pull/3925)

* Support for disabling libunwind-based backtraces of WebAssembly code at
  compile time has been added.
  [#3932](https://github.com/bytecodealliance/wasmtime/pull/3932)

* Async support for call hooks has been added to optionally execute "blocking"
  work whenever a wasm module is entered or exited relative to the host.
  [#3876](https://github.com/bytecodealliance/wasmtime/pull/3876)

### Fixed

* Loading a `Module` will now check, at runtime, that the compilation settings
  enabled in a `Config` are compatible with the native host. For example this
  ensures that if avx2 is enabled that the host actually has avx2 support.
  [#3899](https://github.com/bytecodealliance/wasmtime/pull/3899)

### Removed

* Support for `Config::interruptable` and `InterruptHandle` has been removed
  from the `wasmtime` crate. Users should migrate to using epoch-based
  interruption instead.
  [#3925](https://github.com/bytecodealliance/wasmtime/pull/3925)

* The module linking implementation of Wasmtime has been removed to make room
  for the upcoming support for the component model.
  [#3958](https://github.com/bytecodealliance/wasmtime/pull/3958)

--------------------------------------------------------------------------------

## 0.35.3

Released 2022-04-11.

### Fixed

* Backported a bugfix for an instruction lowering issue that could cause a
  regalloc panic due to an undefined register in some cases. No miscompilation
  was ever possible, but panics would result in a compilation failure.
  [#4012](https://github.com/bytecodealliance/wasmtime/pull/4012)

--------------------------------------------------------------------------------

## 0.35.2

Released 2022-03-31.

### Security Fixes

* [CVE-2022-24791](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-gwc9-348x-qwv2):
  Fixed a use after free with `externref`s and epoch interruption.

## 0.35.1

Released 2022-03-09.

### Fixed

* Fixed a bug in the x86-64 lowering of the `uextend` opcode for narrow (`i8`,
  `i16`) integer sources when the value is produced by one of several
  arithmetic instructions.
  [#3906](https://github.com/bytecodealliance/wasmtime/pull/3906)

## 0.35.0

Released 2022-03-07.

### Added

* The `wasmtime_wasi::add_to_linker` function now allows providing
  a context object of a custom type instead of `wasmtime_wasi::WasiCtx`,
  as long as that type implements the required WASI snapshot traits.
  This allows, for example, wrapping `WasiCtx` into a struct and providing
  custom implementations for those traits to override the default behaviour.

### Changed

* WebAssembly tables of `funcref` values are now lazily initialized which can,
  in some cases, greatly speed up instantiation of a module.
  [#3733](https://github.com/bytecodealliance/wasmtime/pull/3733)

* The `memfd` feature in 0.34.0, now renamed to `memory-init-cow`, has been
  enabled by default. This means that, where applicable, WebAssembly linear
  memories are now initialized with copy-on-write mappings. Support from this
  has been expanded from Linux-only to include macOS and other Unix systems when
  modules are loaded from precompiled `*.cwasm` files on disk.
  [#3777](https://github.com/bytecodealliance/wasmtime/pull/3777)
  [#3778](https://github.com/bytecodealliance/wasmtime/pull/3778)
  [#3787](https://github.com/bytecodealliance/wasmtime/pull/3787)
  [#3819](https://github.com/bytecodealliance/wasmtime/pull/3819)
  [#3831](https://github.com/bytecodealliance/wasmtime/pull/3831)

* Clarify that SSE 4.2 (and prior) is required for running WebAssembly code with
  simd support enabled on x86\_64.
  [#3816](https://github.com/bytecodealliance/wasmtime/pull/3816)
  [#3817](https://github.com/bytecodealliance/wasmtime/pull/3817)
  [#3833](https://github.com/bytecodealliance/wasmtime/pull/3833)
  [#3825](https://github.com/bytecodealliance/wasmtime/pull/3825)

* Support for profiling with VTune is now enabled at compile time by default,
  but it remains disabled at runtime by default.
  [#3821](https://github.com/bytecodealliance/wasmtime/pull/3821)

* The `ModuleLimits` type has been removed from the configuration of the pooling
  allocator in favor of configuring the total size of an instance allocation
  rather than each individual field.
  [#3837](https://github.com/bytecodealliance/wasmtime/pull/3837)

* The native stack size allowed for WebAssembly has been decreased from 1 MiB to
  512 KiB on all platforms to better accomodate running wasm on the main thread
  on Windows.
  [#3861](https://github.com/bytecodealliance/wasmtime/pull/3861)

* The `wasi-common` crate now supports doing polls for both read and write
  interest on a file descriptor at the same time.
  [#3866](https://github.com/bytecodealliance/wasmtime/pull/3866)

### Fixed

* The `Store::call_hook` callback is now invoked when entering host functions
  defined with `*_unchecked` variants.
  [#3881](https://github.com/bytecodealliance/wasmtime/pull/3881)

### Removed

* The incomplete and unmaintained ARM32 backend has been removed from Cranelift.
  [#3799](https://github.com/bytecodealliance/wasmtime/pull/3799)

--------------------------------------------------------------------------------

## 0.34.2

Released 2022-03-31.

### Security Fixes

* [CVE-2022-24791](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-gwc9-348x-qwv2):
  Fixed a use after free with `externref`s and epoch interruption.

## 0.34.1

Released 2022-02-16.

### Security Fixes

* [CVE-2022-23636](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-88xq-w8cq-xfg7):
  Fixed an invalid drop of a partially-initialized instance in the pooling instance
  allocator.

## 0.33.1

Released 2022-02-16.

### Security Fixes

* [CVE-2022-23636](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-88xq-w8cq-xfg7):
  Fixed an invalid drop of a partially-initialized instance in the pooling instance
  allocator.

## 0.34.0

Released 2022-02-07.

### Fixed

* The `wasi-common` default implementation of some attributes of files has been
  updated to ensure that `wasi-libc`'s `isatty` function works as intended.
  [#3696](https://github.com/bytecodealliance/wasmtime/pull/3696)

* A benign debug assertion related to `externref` and garbage-collection has
  been fixed.
  [#3734](https://github.com/bytecodealliance/wasmtime/pull/3734)

### Added

* Function names are now automatically demangled when informing profilers of
  regions of JIT code to apply Rust-specific demangling rules if applicable.
  [#3683](https://github.com/bytecodealliance/wasmtime/pull/3683)

* Support for profiling JIT-generated trampolines with VTune has been added.
  [#3687](https://github.com/bytecodealliance/wasmtime/pull/3687)

* Wasmtime now supports a new method of async preemption dubbed "epoch-based
  interruption" which is intended to be much more efficient than the current
  fuel-based method of preemption.
  [#3699](https://github.com/bytecodealliance/wasmtime/pull/3699)

* On Linux Wasmtime will now by default use copy-on-write mappings to initialize
  memories of wasm modules where possible, accelerating instantiation by
  avoiding costly memory copies. When combined with the pooling allocator this
  can also be used to speed up instance-reuse cases due to fewer syscalls to
  change memory mappings being necessary.
  [#3697](https://github.com/bytecodealliance/wasmtime/pull/3697)
  [#3738](https://github.com/bytecodealliance/wasmtime/pull/3738)
  [#3760](https://github.com/bytecodealliance/wasmtime/pull/3760)

* Wasmtime now supports the recently-added `sock_accept` WASI function.
  [#3711](https://github.com/bytecodealliance/wasmtime/pull/3711)

* Cranelift now has support for specifying blocks as cold.
  [#3698](https://github.com/bytecodealliance/wasmtime/pull/3698)

### Changed

* Many more instructions for the x64 backend have been migrated to ISLE,
  additionally with refactorings to make incorrect lowerings harder to
  accidentally write.
  [#3653](https://github.com/bytecodealliance/wasmtime/pull/3653)
  [#3659](https://github.com/bytecodealliance/wasmtime/pull/3659)
  [#3681](https://github.com/bytecodealliance/wasmtime/pull/3681)
  [#3686](https://github.com/bytecodealliance/wasmtime/pull/3686)
  [#3688](https://github.com/bytecodealliance/wasmtime/pull/3688)
  [#3690](https://github.com/bytecodealliance/wasmtime/pull/3690)
  [#3752](https://github.com/bytecodealliance/wasmtime/pull/3752)

* More instructions in the aarch64 backend are now lowered with ISLE.
  [#3658](https://github.com/bytecodealliance/wasmtime/pull/3658)
  [#3662](https://github.com/bytecodealliance/wasmtime/pull/3662)

* The s390x backend's lowering rules are now almost entirely defined with ISLE.
  [#3702](https://github.com/bytecodealliance/wasmtime/pull/3702)
  [#3703](https://github.com/bytecodealliance/wasmtime/pull/3703)
  [#3706](https://github.com/bytecodealliance/wasmtime/pull/3706)
  [#3717](https://github.com/bytecodealliance/wasmtime/pull/3717)
  [#3723](https://github.com/bytecodealliance/wasmtime/pull/3723)
  [#3724](https://github.com/bytecodealliance/wasmtime/pull/3724)

* Instantiation of modules in Wasmtime has been further optimized now that the
  copy-on-write memory initialization removed the previously most-expensive part
  of instantiating a module.
  [#3727](https://github.com/bytecodealliance/wasmtime/pull/3727)
  [#3739](https://github.com/bytecodealliance/wasmtime/pull/3739)
  [#3741](https://github.com/bytecodealliance/wasmtime/pull/3741)
  [#3742](https://github.com/bytecodealliance/wasmtime/pull/3742)

--------------------------------------------------------------------------------

## 0.33.0

Released 2022-01-05.

### Added

* Compiled wasm modules may now optionally omit debugging information about
  mapping addresses to source locations, resulting in smaller binaries.
  [#3598](https://github.com/bytecodealliance/wasmtime/pull/3598)

* The WebAssembly SIMD proposal is now enabled by default.
  [#3601](https://github.com/bytecodealliance/wasmtime/pull/3601)

--------------------------------------------------------------------------------

## 0.32.1

Released 2022-01-04.

### Fixed

* Cranelift: remove recently-added build dependency on `sha2` to allow usage in
  some dependency-sensitive environments, by computing ISLE manifest hashes
  with a different hash function.
  [#3619](https://github.com/bytecodealliance/wasmtime/pull/3619)

* Cranelift: fixed 8- and 16-bit behavior of popcount (bit population count)
  instruction. Does not affect Wasm frontend.
  [#3617](https://github.com/bytecodealliance/wasmtime/pull/3617)

* Cranelift: fixed miscompilation of 8- and 16-bit bit-rotate instructions.
  Does not affect Wasm frontend.
  [#3610](https://github.com/bytecodealliance/wasmtime/pull/3610)

--------------------------------------------------------------------------------

## 0.32.0

Released 2021-12-13.

### Added

* A new configuration option has been added to force using a "static" memory
  style to automatically limit growth of memories in some configurations.
  [#3503](https://github.com/bytecodealliance/wasmtime/pull/3503)

* The `InstancePre<T>` type now implements `Clone`.
  [#3510](https://github.com/bytecodealliance/wasmtime/pull/3510)

* Cranelift's instruction selection process has begun to be migrated towards the
  ISLE compiler and definition language.
  [#3506](https://github.com/bytecodealliance/wasmtime/pull/3506)

* A `pooling-allocator` feature has been added, which is on-by-default, to
  disable the pooling allocator at compile time.
  [#3514](https://github.com/bytecodealliance/wasmtime/pull/3514)

### Fixed

* A possible panic when parsing a WebAssembly `name` section has been fixed.
  [#3509](https://github.com/bytecodealliance/wasmtime/pull/3509)

* Generating native DWARF information for some C-produced modules has been
  fixed, notably those where there may be DWARF about dead code.
  [#3498](https://github.com/bytecodealliance/wasmtime/pull/3498)

* A number of SIMD code generation bugs have been fixed in the x64 backend
  by migrating their lowerings to ISLE.

--------------------------------------------------------------------------------

## 0.31.0

Released 2021-10-29.

### Added

* New `Func::new_unchecked` and `Func::call_unchecked` APIs have been added with
  accompanying functions in the C API to improve the performance of calls into
  wasm and the host in the C API.
  [#3350](https://github.com/bytecodealliance/wasmtime/pull/3350)

* Release binaries are now available for the s390x-unknown-linux-gnu
  architecture.
  [#3372](https://github.com/bytecodealliance/wasmtime/pull/3372)

* A new `ResourceLimiterAsync` trait is added which allows asynchronous blocking
  of WebAssembly on instructions such as `memory.grow`.
  [#3393](https://github.com/bytecodealliance/wasmtime/pull/3393)

### Changed

* The `Func::call` method now takes a slice to write the results into rather
  than returning a boxed slice.
  [#3319](https://github.com/bytecodealliance/wasmtime/pull/3319)

* Trampolines are now covered when jitdump profiling is enabled.
  [#3344](https://github.com/bytecodealliance/wasmtime/pull/3344)

### Fixed

* Debugging with GDB has been fixed on Windows.
  [#3373](https://github.com/bytecodealliance/wasmtime/pull/3373)

* Some quadradic behavior in Wasmtime's compilation of modules has been fixed.
  [#3469](https://github.com/bytecodealliance/wasmtime/pull/3469)
  [#3466](https://github.com/bytecodealliance/wasmtime/pull/3466)

* Bounds-checks for wasm memory accesses in certain non-default configurations
  have been fixed to correctly allow loads at the end of the address space.
  [#3462](https://github.com/bytecodealliance/wasmtime/pull/3462)

* When type-checking memories and tables for satisfying instance imports the
  runtime size of the table/memory is now consulted instead of the object's
  original type.
  [#3450](https://github.com/bytecodealliance/wasmtime/pull/3450)

### Removed

* The Lightbeam backend has been removed, as per [RFC 14].
  [#3390](https://github.com/bytecodealliance/wasmtime/pull/3390)

[RFC 14]: https://github.com/bytecodealliance/rfcs/pull/14

* Cranelift's old x86 backend has been removed, as per [RFC 12].
  [#3309](https://github.com/bytecodealliance/wasmtime/pull/3009)

[RFC 12]: https://github.com/bytecodealliance/rfcs/pull/12

## 0.30.0

Released 2021-09-17.

### Security Fixes

* [CVE-2021-39216](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-v4cp-h94r-m7xf):
  Fixed a use after free passing `externref`s to Wasm in Wasmtime.

* [CVE-2021-39218](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-4873-36h9-wv49):
  Fixed an out-of-bounds read/write and invalid free with `externref`s and GC
  safepoints in Wasmtime.

* [CVE-2021-39219](https://github.com/bytecodealliance/wasmtime/security/advisories/GHSA-q879-9g95-56mx):
  Fixed a bug where using two different `Engine`s with the same `Linker`-define
  functions caused unsafety without `unsafe` blocks.

### Added

* Added experimental support for the in-progress 64-bit memories Wasm proposal.

* Added support to build Wasmtime without the compiler. This lets you run
  pre-compiled Wasm modules, without the ability (or potential attack surface)
  of compiling new Wasm modules. The compilation functionality is gated by the
  on-by-default `cranelift` cargo feature.

* Added support for NaN canonicalization with SIMD vectors.

* Added support for differential fuzzing against V8's Wasm engine.

* Added support for fuzzing against the Wasm spec interpreter.

* Enabled SIMD fuzzing on oss-fuzz.

### Changed

* A variety of performance improvements to loading pre-compiled modules.

* A variety of performance improvements to function calls, both through Rust and
  the C API.

* Leaf functions that do not use the stack no longer bump the frame pointer on
  aarch64 and s390x.

* Many updates and expanded instruction support to the in-progress CLIF
  interpreter.

* Expanded fuzzing of reference types and GC.

### Fixed

* A number of fixes to both aarch64 and x86_64 support for the Wasm SIMD
  proposal and the underlying CLIF vector instructions.

* Fixed a potential infinite loop in the SSA computation for
  `cranelift-frontend`. This was not reachable from `cranelift-wasm` or
  Wasmtime, but might have affected general Cranelift users.

### Removed

* The `wasmtime wasm2obj` subcommand has been removed. Generating raw object
  files for linking natively is no longer supported. Use the `wasmtime compile`
  subcommand to pre-compile a Wasm module and `wasmtime run` to run pre-compiled
  Wasm modules.

## 0.29.0

Released 2021-08-02.

### Changed

* Instance exports are now loaded lazily from instances instead of eagerly as
  they were before. This is an internal-only change and is not a breaking
  change.
  [#2984](https://github.com/bytecodealliance/wasmtime/pull/2984)

* All linear memories created by Wasmtime will now, by default, have guard pages
  in front of them in addition to after them. This is intended to help mitigate
  future bugs in Cranelift, should they arise.
  [#2977](https://github.com/bytecodealliance/wasmtime/pull/2977)

* Linear memories now correctly support a maximum size of 4GB. Previously, the
  limit field was 32 bits, which did not properly support a full 4GB memory.
  This update is also a necessary change in preparation for future memory64
  support.
  [#3013](https://github.com/bytecodealliance/wasmtime/pull/3013)
  [#3134](https://github.com/bytecodealliance/wasmtime/pull/3134)

* Injection counts of fuel into a `wasmtime::Store` now uses a u64 instead of a
  u32.
  [#3048](https://github.com/bytecodealliance/wasmtime/pull/3048)

### Added

* Support for `i128` has improved in the AArch64 backend.
  [#2959](https://github.com/bytecodealliance/wasmtime/pull/2959)
  [#2975](https://github.com/bytecodealliance/wasmtime/pull/2975)
  [#2985](https://github.com/bytecodealliance/wasmtime/pull/2985)
  [#2990](https://github.com/bytecodealliance/wasmtime/pull/2990)
  [#3002](https://github.com/bytecodealliance/wasmtime/pull/3002)
  [#3004](https://github.com/bytecodealliance/wasmtime/pull/3004)
  [#3005](https://github.com/bytecodealliance/wasmtime/pull/3005)
  [#3008](https://github.com/bytecodealliance/wasmtime/pull/3008)
  [#3027](https://github.com/bytecodealliance/wasmtime/pull/3027)

* The s390x backend now supports z14 and atomics.
  [#2988](https://github.com/bytecodealliance/wasmtime/pull/2988)
  [#2991](https://github.com/bytecodealliance/wasmtime/pull/2991)

* The `wasmtime::Linker` type now implements `Clone`.
  [#2993](https://github.com/bytecodealliance/wasmtime/pull/2993)

* Support for the SIMD proposal on both x86\_64 and AArch64 has improved. On
  x86\_64, all SIMD opcodes are now supported.
  [#2997](https://github.com/bytecodealliance/wasmtime/pull/2997)
  [#3035](https://github.com/bytecodealliance/wasmtime/pull/3035)
  [#2982](https://github.com/bytecodealliance/wasmtime/pull/2982)
  [#3084](https://github.com/bytecodealliance/wasmtime/pull/3084)
  [#3082](https://github.com/bytecodealliance/wasmtime/pull/3082)
  [#3107](https://github.com/bytecodealliance/wasmtime/pull/3107)
  [#3105](https://github.com/bytecodealliance/wasmtime/pull/3105)
  [#3114](https://github.com/bytecodealliance/wasmtime/pull/3114)
  [#3070](https://github.com/bytecodealliance/wasmtime/pull/3070)
  [#3126](https://github.com/bytecodealliance/wasmtime/pull/3126)

* A `Trap` can now display its reason without also displaying the backtrace.
  [#3033](https://github.com/bytecodealliance/wasmtime/pull/3033)

* An initiall fuzzer for CLIF has been added.
  [#3038](https://github.com/bytecodealliance/wasmtime/pull/3038)

* High-level architecture documentation has been added for Wasmtime.
  [#3019](https://github.com/bytecodealliance/wasmtime/pull/3019)

* Support for multi-memory can now be configured in Wasmtime's C API.
  [#3071](https://github.com/bytecodealliance/wasmtime/pull/3071)

* The `wasmtime` crate now supports a `posix-signals-on-macos` feature to force
  the usage of signals instead of mach ports to handle traps on macOS.
  [#3063](https://github.com/bytecodealliance/wasmtime/pull/3063)

* Wasmtime's C API now has a `wasmtime_trap_code` function to get the raw trap
  code, if present, for a trap.
  [#3086](https://github.com/bytecodealliance/wasmtime/pull/3086)

* Wasmtime's C API now has a `wasmtime_linker_define_func` function to define a
  store-independent function within a linker.
  [#3122](https://github.com/bytecodealliance/wasmtime/pull/3122)

* A `wasmtime::Linker::module_async` function was added as the asynchronous
  counterpart to `wasmtime::Linker::module`.
  [#3121](https://github.com/bytecodealliance/wasmtime/pull/3121)

### Fixed

* Compiling the `wasmtime` crate into a `dylib` crate type has been fixed.
  [#3010](https://github.com/bytecodealliance/wasmtime/pull/3010)

* The enter/exit hooks for WebAssembly are now executed for an instance's
  `start` function, if present.
  [#3001](https://github.com/bytecodealliance/wasmtime/pull/3001)

* Some WASI functions in `wasi-common` have been fixed for big-endian platforms.
  [#3016](https://github.com/bytecodealliance/wasmtime/pull/3016)

* Wasmtime no longer erroneously assumes that all custom sections may contain
  DWARF information, reducing instances of `Trap`'s `Display` implementation
  providing misleading information to set an env var to get more information.
  [#3083](https://github.com/bytecodealliance/wasmtime/pull/3083)

* Some issues with parsing DWARF debug information have been fixed.
  [#3116](https://github.com/bytecodealliance/wasmtime/pull/3116)

## 0.28.0

Released 2021-06-09.

### Changed

* Breaking: Wasmtime's embedding API has been redesigned, as specified in [RFC
  11]. Rust users can now enjoy easier times with `Send` and `Sync`, and all
  users can now more clearly manage memory, especially in the C API. Language
  embeddings have been updated to the new API as well.
  [#2897](https://github.com/bytecodealliance/wasmtime/pull/2897)

[RFC 11]: https://github.com/bytecodealliance/rfcs/pull/11

### Added

* A new `InstancePre` type, created with `Linker::instantiate_pre`, has been
  added to perform type-checking of an instance once and reduce the work done
  for each instantiation of a module:
  [#2962](https://github.com/bytecodealliance/wasmtime/pull/2962)

* Deserialization of a module can now optionally skip checking the wasmtime
  version string:
  [#2945](https://github.com/bytecodealliance/wasmtime/pull/2945)

* A method has been exposed to frontload per-thread initialization costs if the
  latency of every last wasm call is important:
  [#2946](https://github.com/bytecodealliance/wasmtime/pull/2946)

* Hooks have been added for entry/exit into wasm code to allow embeddings to
  track time and other properties about execution in a wasm environment:
  [#2952](https://github.com/bytecodealliance/wasmtime/pull/2952)

* A [C++ embedding of Wasmtime has been written][cpp].

[RFC 11]: https://github.com/bytecodealliance/rfcs/pull/11
[cpp]: https://github.com/bytecodealliance/wasmtime-cpp

### Fixed

* Multiple returns on macOS AArch64 have been fixed:
  [#2956](https://github.com/bytecodealliance/wasmtime/pull/2956)

## 0.27.0

Released 2021-05-21.

### Security Fixes

* Fixed a security issue in Cranelift's x64 backend that could result in a heap
  sandbox escape due to an incorrect sign-extension:
  [#2913](https://github.com/bytecodealliance/wasmtime/issues/2913).

### Added

* Support for IBM z/Archiecture (`s390x`) machines in Cranelift and Wasmtime:
  [#2836](https://github.com/bytecodealliance/wasmtime/pull/2836),
  [#2837](https://github.com/bytecodealliance/wasmtime/pull/2837),
  [#2838](https://github.com/bytecodealliance/wasmtime/pull/2838),
  [#2843](https://github.com/bytecodealliance/wasmtime/pull/2843),
  [#2854](https://github.com/bytecodealliance/wasmtime/pull/2854),
  [#2870](https://github.com/bytecodealliance/wasmtime/pull/2870),
  [#2871](https://github.com/bytecodealliance/wasmtime/pull/2871),
  [#2872](https://github.com/bytecodealliance/wasmtime/pull/2872),
  [#2874](https://github.com/bytecodealliance/wasmtime/pull/2874).

* Improved async support in wasi-common runtime:
  [#2832](https://github.com/bytecodealliance/wasmtime/pull/2832).

* Added `Store::with_limits`, `StoreLimits`, and `ResourceLimiter` to the
  Wasmtime API to help with enforcing resource limits at runtime. The
  `ResourceLimiter` trait can be implemented by custom resource limiters to
  decide if linear memories or tables can be grown.

* Added `allow-unknown-exports` option for the run command:
  [#2879](https://github.com/bytecodealliance/wasmtime/pull/2879).

* Added API to notify that a `Store` has moved to a new thread:
  [#2822](https://github.com/bytecodealliance/wasmtime/pull/2822).

* Documented guidance around using Wasmtime in multithreaded contexts:
  [#2812](https://github.com/bytecodealliance/wasmtime/pull/2812).
  In the future, the Wasmtime API will change to allow some of its core types
  to be Send/Sync; see the in-progress
  [#2897](https://github.com/bytecodealliance/wasmtime/pull/2897) for details.

* Support calls from native code to multiple-return-value functions:
  [#2806](https://github.com/bytecodealliance/wasmtime/pull/2806).

### Changed

* Breaking: `Memory::new` has been changed to return `Result` as creating a
  host memory object is now a fallible operation when the initial size of
  the memory exceeds the store limits.

### Fixed

* Many instruction selection improvements on x64 and aarch64:
  [#2819](https://github.com/bytecodealliance/wasmtime/pull/2819),
  [#2828](https://github.com/bytecodealliance/wasmtime/pull/2828),
  [#2823](https://github.com/bytecodealliance/wasmtime/pull/2823),
  [#2862](https://github.com/bytecodealliance/wasmtime/pull/2862),
  [#2886](https://github.com/bytecodealliance/wasmtime/pull/2886),
  [#2889](https://github.com/bytecodealliance/wasmtime/pull/2889),
  [#2905](https://github.com/bytecodealliance/wasmtime/pull/2905).

* Improved performance of Wasmtime runtime substantially:
  [#2811](https://github.com/bytecodealliance/wasmtime/pull/2811),
  [#2818](https://github.com/bytecodealliance/wasmtime/pull/2818),
  [#2821](https://github.com/bytecodealliance/wasmtime/pull/2821),
  [#2847](https://github.com/bytecodealliance/wasmtime/pull/2847),
  [#2900](https://github.com/bytecodealliance/wasmtime/pull/2900).

* Fixed WASI issue with file metadata on Windows:
  [#2884](https://github.com/bytecodealliance/wasmtime/pull/2884).

* Fixed an issue with debug info and an underflowing (trapping) offset:
  [#2866](https://github.com/bytecodealliance/wasmtime/pull/2866).

* Fixed an issue with unwind information in the old x86 backend:
  [#2845](https://github.com/bytecodealliance/wasmtime/pull/2845).

* Fixed i32 spilling in x64 backend:
  [#2840](https://github.com/bytecodealliance/wasmtime/pull/2840).

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
