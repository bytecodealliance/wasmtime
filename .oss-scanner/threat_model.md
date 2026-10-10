# Wasmtime threat model

This document is for Anthropic's OSS Scanner. It summarizes, and links to,
Wasmtime's own security documentation. Where they disagree, the documentation
in `docs/` wins, in particular:

* `docs/security-what-is-considered-a-security-vulnerability.md`
* `docs/stability-tiers.md` and `docs/stability-wasm-proposals.md`
* `docs/security.md`

## What this project does and where untrusted input enters

Wasmtime is a runtime for WebAssembly (Wasm) modules and components. It is
embedded in other programs through the `wasmtime` Rust crate and the C API
(`crates/c-api`), and is also shipped as the `wasmtime` CLI. Its central
promise is that a Wasm guest cannot escape its sandbox, no matter what code it
contains, and cannot harm the host except in ways the embedder explicitly
allows.

**Untrusted** (attacker-controlled):

* The bytes of any Wasm module or component passed to `Module::new`,
  `Component::new`, `wasmtime run`, `wasmtime serve`, etc., in binary or text
  form. This covers validation, translation, compilation (Cranelift and
  Winch), instantiation, and execution.
* Everything a running guest does: the instructions it executes, the values it
  passes to imports (including WASI) and returns from exports, the order in
  which it calls things, and how it uses component-model resources, streams,
  futures, and async tasks.
* Data a guest receives from the outside through WASI (file contents, socket
  data, HTTP requests and responses in `wasmtime serve`/`wasi-http`).

**Trusted** (not part of the attack surface):

* The embedder: its Rust or C code, the host functions it defines, the
  `Config` it chooses, and the arguments and flags given to the `wasmtime`
  CLI.
* Precompiled artifacts. `Module::deserialize`, `Component::deserialize`, and
  `wasmtime run --allow-precompiled` load native code and are documented as
  `unsafe` for untrusted input. The same goes for the compilation cache
  directory.
* The host's filesystem layout, environment variables, and the directories
  the embedder chooses to preopen for WASI.

Bugs must be reachable through Wasmtime's public API (the `wasmtime` crate,
the C API, or the CLI). Code that uses `unsafe` public APIs in ways their
documentation forbids, or that calls crate-private functions directly, does
not count.

## Components that matter most / least

Only Tier 1 platforms and features can have security vulnerabilities (see
`docs/stability-tiers.md`). The scanner's environment is
`x86_64-unknown-linux-gnu`, which is Tier 1.

Most important:

* **Cranelift's x86-64 backend and the Wasm-to-CLIF translation**
  (`cranelift/codegen` incl. its ISLE lowering rules and mid-end
  optimizations, `crates/cranelift`, `crates/environ`). A miscompilation that
  lets a guest access memory outside its linear memories, tables, or GC heap,
  or bypass Wasm's control-flow integrity, is a sandbox escape. Particular
  attention: bounds checks and address computation (sign/zero extension,
  `memory64`, guard-page assumptions), `call_indirect`/`call_ref` type checks,
  table accesses, GC object field accesses, and tail calls.
* **Winch** (`winch/`), Wasmtime's baseline compiler, on x86-64: the same
  concerns as Cranelift.
* **Cranelift's aarch64 backend**
  While the aarch64 backend isn't officially tier 1, it's approaching it, so
  we want to treat issues in it with high priority, with the same concerns
  as the x86-64 backend.
* **The runtime** (`crates/wasmtime/src/runtime`): instance and `VMContext`
  layout, linear memories and tables (including growth), libcalls, traps and
  signal handling, stack-overflow detection, fuel and epoch interruption, the
  pooling allocator (including reuse of memory/tables/stacks between instances
  and leaking data across them), and the GC heap and collectors.
* **The component model** (`crates/wasmtime/src/runtime/component`,
  `crates/environ/src/fact`): lifting and lowering of values across the
  canonical ABI, string transcoding, resource tables and handles, and the
  on-by-default async ABI (tasks, streams, futures, waitables). Confusion
  between handles of different types or owners, and host memory unsafety
  driven by guest-supplied lengths or pointers, are high value.
* **Component model async support**
  (`crates/wasmtime/src/runtime/component/concurrent.rs` and
  `crates/wasmtime/src/runtime/component/concurrent`): everything part of the
  async runtime is high value, but in particular anything that can cause panics
  or deadlocks.
* **WASI** (`crates/wasi`, `crates/wasi-http`, `crates/wasi-io`, and the
  preview1 adapter): capability enforcement (access to files outside preopened
  directories, symlink and `..` handling, use of sockets without permission),
  host-side resource consumption driven by the guest, and host panics.
* **The C API** (`crates/c-api`): memory safety of the API as documented.

Lower value, and **not** security vulnerabilities (Tier 2 or 3, or explicitly
excluded):

* Other targets: s390x, riscv64, Pulley (the interpreter), `no_std`, 32-bit 
  hosts, Windows-GNU, and the C++ API.
* The `threads` proposal (shared memories, atomics), `custom-page-sizes`,
  `stack-switching`, `branch-hinting`, `compact-imports`, and off-by-default
  component-model features (stackful async, threading, map, implements, and
  similar). See `docs/stability-wasm-proposals.md`.
* `wasi-nn`, `wasi-threads`, `wasi-config`, `wasi-keyvalue`, `wasi-tls`.
* DWARF debug info, guest debugging (`crates/debugger`, gdbstub), profiling
  integrations, and coredumps.
* Cranelift used directly, outside Wasmtime, with CLIF that Wasm translation
  can never produce (e.g. odd integer types or non-Wasmtime calling
  conventions).
* Test code, fuzzing infrastructure (`crates/fuzzing`, `fuzz/`), examples,
  benches, `ci/`, `scripts/`, and the ISLE verifier (`cranelift/isle/veri`).

Bugs in Tier 2 features are still bugs that we want fixed. Report them only if
they are serious, rated Low, and clearly marked as affecting a Tier 2 feature.
Do not report Tier 3 issues.

Any combination of `Config` settings that `Engine::new` accepts is in scope,
as long as the features involved are Tier 1. Say in the report which settings
are needed; a bug that needs an unusual configuration is less severe than one
that triggers with the defaults.

## How to exercise it

The image has everything prebuilt in the dev profile (debug assertions
enabled), and Cargo runs offline (`CARGO_NET_OFFLINE=true`):

* `target/debug/wasmtime`, the CLI, and `target/release/wasmtime`, the same
  without debug assertions. Useful subcommands:
  `wasmtime run`, `wasmtime wast`, `wasmtime serve`, and `wasmtime compile`
  (`--emit-clif <dir>` dumps the CLIF). Configuration flags are grouped as
  `-W` (Wasm features, e.g. `-W gc=y`), `-C` (codegen, e.g.
  `-C compiler=winch`), `-O` (optimization and memory settings, e.g.
  `-O pooling-allocator=y`, `-O memory-reservation=...`), and `-S` (WASI).
  `wasmtime <subcommand> -W help` (and `-C help`, etc.) lists all options.
  The CLI accepts `.wat` text directly.
* `cargo test --test wast [filter]` runs the spec tests plus
  `tests/misc_testsuite/**/*.wast` under each compiler (Cranelift, Winch,
  Pulley), with and without the pooling allocator, and with each GC collector.
  `;;! name = true` lines at the top of a `.wast` file enable features (see
  `crates/test-util/src/wast.rs`).
* `cargo test --test all [filter]` runs the integration tests in `tests/all/`,
  which use the public `wasmtime` API.
* `target/debug/clif-util test <file.clif>` runs Cranelift filetests
  (`cranelift/filetests/filetests/`), including `test run` with the CLIF
  interpreter as a differential oracle.
* `./ci/run-tests.py [cargo test args]` runs the whole workspace's tests with
  all features enabled, as CI does. These are prebuilt too.
* The fuzz targets in `fuzz/fuzz_targets/` are prebuilt with ASan in
  `target/x86_64-unknown-linux-gnu/release/`; run them with
  `cargo +nightly-2026-07-09 fuzz run <target>`. `differential` (compares
  against the spec interpreter and V8), `instantiate`, `component_api`,
  `gc_ops`, and `call_async` are good starting points.
* `wasm-tools` is installed, for `wasm-tools shrink` (test-case reduction),
  `wasm-tools smith`, `print`, and `validate`.

`.claude/skills/` contains guides used by Wasmtime maintainers for this kind of
work: `wasmtime-auditor`, `cranelift-auditor`, and `reduce-test-cases`. They
are useful for methodology; ignore their instructions about where to write
reports.

The agent machine is small, so prefer adding repro cases inside the workspace
(a `.wast` file in `tests/misc_testsuite/`, or a test in `tests/all/`) over new
crates; those reuse the prebuilt artifacts and `Cargo.lock`. Running
`cargo test -p <crate>` for an individual crate can trigger a long rebuild
because the feature set differs.

## How you rate severity

The impact on an embedder running an untrusted guest is what counts.
Reproduce in release mode where possible: a bug that needs debug assertions to
show up has no release-mode impact unless you show one.

* **Critical**: a guest-controlled sandbox escape with the default
  configuration or a commonly used one (pooling allocator, async with
  fuel/epochs, Winch): arbitrary read/write of host memory, control of host
  execution, or a use-after-free that a guest can trigger and steer.
  Typical sources: miscompiled or elided bounds checks, broken CFI, type
  confusion in GC or `funcref`s, freed host memory reachable from Wasm.
* **High**: a sandbox escape that needs a less common but supported
  configuration; a guest reading host memory or another instance's data
  (including stale data in a reused pooling-allocator slot); WASI capability
  bypasses, such as reading or writing files outside the preopened
  directories; host memory unsafety reachable from safe embedder code
  driven by guest-controlled values.
* **Medium**: denial of service against the host that a guest triggers at
  run time: a host panic or abort, an infinite loop that fuel or epoch
  interruption does not stop when configured, or memory or other resource
  exhaustion that escapes configured limits (`ResourceLimiter`, `StoreLimits`,
  pooling limits) or grows without bound in the host (e.g. in WASI
  implementations). Memory unsafety that needs unusual but safe embedder API
  use, with no guest control, also belongs here.
* **Low**: issues that need unlikely configurations or embedder behavior and
  have limited impact; defense-in-depth weaknesses (e.g. a guard region that
  is smaller than documented) with no demonstrated exploit; serious bugs in
  Tier 2 features (marked as such).

**Not vulnerabilities** but worth reporting as normal bugs:

* Anything during *compilation* that is not memory unsafety: panics, slow
  compilation, excessive memory use while compiling, infinite loops in the
  register allocator, and so on.
* Wasm executing with the wrong semantics but staying inside the sandbox:
  wrong results, spurious or missing traps.

**Not vulnerabilities** and not worth reporting at all:

* Behavior that Wasm allows to differ between engines: NaN bit patterns,
  relaxed-SIMD results, how deep recursion can go before a stack overflow,
  whether `memory.grow`/`table.grow` succeed. WASIp1 error codes that differ
  from other engines.
* Memory or CPU use by a guest that stays within the limits the embedder
  configured, or that is unlimited because the embedder set no limits.
* Bugs that need untrusted precompiled artifacts or cache contents, untrusted
  CLI flags or `Config`, misuse of `unsafe` APIs, or bugs in host functions
  written by the embedder.
* Spectre-style side channels beyond the mitigations described in
  `docs/security.md`.

## How reports and patches should look

* A minimal reproducer, in order of preference: a `.wast` file (with the
  `wasmtime wast` flags needed listed in a comment at the top), a `.wat` or
  `.wasm` for `wasmtime run` plus the exact command line, or a Rust `#[test]`
  that uses only the public `wasmtime` API. For Cranelift-only issues, a
  `.clif` file runnable with `clif-util` also helps, but should come with a Wasm
  reproducer that shows the bug is reachable from Wasm.
* The exact configuration: CLI flags or `Config` calls, Cargo features, and
  whether it reproduces in release mode.
* The commit you tested, and the expected and actual behavior (the trap,
  panic message, or sanitizer and `gdb` output).
* One report per root cause. Fuzzers find the same bug in many shapes, so
  deduplicate first.
* Patches should be minimal, fix the root cause rather than the symptom, and
  add a regression test (usually a `.wast` file in `tests/misc_testsuite/` or a
  test in `tests/all/`).

## Anything to leave alone

* `debug_assert!` failures, `unreachable!()`s, or `todo!()`s in code that the
  public API cannot reach.
* Known shortcomings documented in comments or in the docs listed above,
  unless you can combine them with another bug into a real vulnerability.
* Code that is only compiled for Tier 2 or Tier 3 platforms (s390x,
  riscv64, Pulley-only paths, etc.). Code specific to the other Tier 1
  platforms (macOS and Windows on x86-64) is in scope, but cannot be run here:
  report such issues only with a strong, concrete argument from the code.
