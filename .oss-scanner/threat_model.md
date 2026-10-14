# Wasmtime threat model

This document is for Anthropic's OSS Scanner. Wasmtime's security policy is
documented in `docs/`; read these first, they take precedence over anything
here, unless otherwise stated explicitly in this document:

* `docs/security-what-is-considered-a-security-vulnerability.md`: what is and
  isn't a vulnerability.
* `docs/stability-tiers.md` and `docs/stability-wasm-proposals.md`: which
  platforms and features are Tier 1. Only Tier 1 can have vulnerabilities.
* `docs/security.md`: sandboxing and defense-in-depth mechanisms.

This file only adds what those don't cover: trust boundaries, priorities,
severity ratings, and how to work in this image.

## What this project does and where untrusted input enters

Wasmtime runs WebAssembly modules and components, embedded via the `wasmtime`
crate or the C API, or through the `wasmtime` CLI.

**Untrusted**: the Wasm module or component (binary or text), across
validation, compilation, instantiation, and execution; everything the guest
does at run time, including values it passes to imports (WASI included) and
how it uses component-model resources and async features; and data reaching
the guest through WASI (files, sockets, HTTP in `wasmtime serve`).

**Trusted**: the embedder (host code, host functions, `Config`, CLI flags);
precompiled artifacts and cache contents (`Module::deserialize` and
`--allow-precompiled` are `unsafe` for untrusted input); and the host
environment, including which directories are preopened for WASI.

Bugs must be reachable through the public API (`wasmtime` crate, C API, CLI)
without violating the contract of any `unsafe` API.

## Components that matter most / least

The scanner runs on `x86_64-unknown-linux-gnu`, a Tier 1 target. Highest
value:

* **Cranelift's x86-64 backend and Wasm-to-CLIF translation**
  (`cranelift/codegen`, `crates/cranelift`, `crates/environ`), and **Winch**
  (`winch/`). Miscompilations that break memory isolation or control-flow
  integrity are sandbox escapes. See `.claude/skills/cranelift-auditor`.
* **Cranelift's aarch64 backend.** While the aarch64 backend isn't officially
  tier 1, it's approaching it, so we want to treat issues in it with high
  priority, with the same concerns as the x86-64 backend. Note that this 
  overrides the policy in the docs.
* **The runtime** (`crates/wasmtime/src/runtime`): memories, tables, GC heap
  and collectors, libcalls, traps, stack-overflow handling, fuel/epochs, and
  the pooling allocator (including data leaking between reused slots).
* **The component model** (`crates/wasmtime/src/runtime/component`,
  `crates/environ/src/fact`): canonical ABI lifting/lowering, string
  transcoding, and resource handles.
* **Component model async support**
  (`crates/wasmtime/src/runtime/component/concurrent.rs` and
  `crates/wasmtime/src/runtime/component/concurrent`): everything part of the
  async runtime is high value, but in particular anything that can cause panics
  or deadlocks.
* **WASI** (`crates/wasi`, `crates/wasi-http`, `crates/wasi-io`):
  capability enforcement and guest-driven host resource consumption.
* **The C API** (`crates/c-api`).

Lower value, and not security vulnerabilities: everything below Tier 1 in the
tier documents (aarch64 above being the exception for prioritization), plus
off-by-default component-model features, test code, fuzzing infrastructure
(`crates/fuzzing`, `fuzz/`), examples, benches, `ci/`, `scripts/`, and
`cranelift/isle/veri`. Report Tier 2 bugs only if they are serious, rated Low,
and clearly marked as Tier 2. Do not report Tier 3 issues.

Any `Config` that `Engine::new` accepts is in scope if the features involved
are Tier 1. A bug that needs an unusual configuration is less severe than one
that triggers with the defaults.

## How to exercise it

General guidance on building, testing, and the CLI is in
`docs/contributing-testing.md`, `docs/contributing-fuzzing.md`,
`docs/contributing-reducing-test-cases.md`, `docs/cli-options.md`, and the
skills in `.claude/skills/` (`wasmtime-auditor`, `cranelift-auditor`,
`reduce-test-cases`; ignore their instructions about where to write reports).

Specific to this image: the following targets are prebuilt.
Other invocations (e.g. `cargo test -p <crate>`) use different feature sets
and can trigger long rebuilds on the small agent machine, so prefer putting
repros in `tests/misc_testsuite/` or `tests/all/`.

* `target/debug/wasmtime` (debug assertions) and `target/release/wasmtime`.
* The repository root's tests with default features (`cargo test`, e.g.
  `cargo test --test wast` or `--test all`).
* `target/debug/clif-util`.
* The whole workspace's tests as CI runs them: `./ci/run-tests.py`.
* The fuzz targets, built with ASan and debug assertions in
  `target/x86_64-unknown-linux-gnu/release/`; run with
  `cargo fuzz run --debug-assertions <target>`.
* `wasm-tools`.

Reproducers you write as `*.wast` files should be run with
`./target/debug/wasmtime wast *.wast`.

## How you rate severity

The impact on an embedder running an untrusted guest is what counts. A bug
that only shows up with debug assertions has no impact unless a release-mode
consequence is shown.

* **Critical**: a guest-controlled sandbox escape with the default
  configuration or a commonly used one (pooling allocator, async with
  fuel/epochs, Winch): arbitrary read/write of host memory, control of host
  execution, or a guest-triggerable use-after-free.
* **High**: a sandbox escape that needs a less common but supported
  configuration; a guest reading host memory or another instance's data; WASI
  capability bypasses; host memory unsafety reachable from safe embedder code
  driven by guest-controlled values.
* **Medium**: run-time denial of service against the host (see the
  vulnerability document), including resource exhaustion that escapes
  configured limits (`ResourceLimiter`, `StoreLimits`, pooling limits) or
  grows without bound in host code such as WASI. Memory unsafety that needs
  unusual but safe embedder API use, with no guest control.
* **Low**: unlikely configurations or embedder behavior with limited impact;
  defense-in-depth weaknesses with no demonstrated exploit; serious bugs in
  Tier 2 features (marked as such).

**Not vulnerabilities** but worth reporting as normal bugs: compile-time
denial of service, and divergence from Wasm semantics that stays inside the
sandbox (see the vulnerability document).

**Not vulnerabilities** and not worth reporting at all:

* Behavior allowed to differ between engines (see "Divergent WebAssembly
  behavior across runtimes" in `docs/contributing-fuzzing.md`).
* Resource use by a guest within the limits the embedder configured, or
  unlimited because the embedder set none.
* Anything that needs untrusted precompiled artifacts, cache contents, CLI
  flags, or `Config`, or bugs in embedder-written host functions.
* Spectre-style side channels beyond the mitigations in `docs/security.md`.

## How reports and patches should look

Follow "Making a bug report" in `.claude/skills/wasmtime-auditor/SKILL.md`
(or `cranelift-auditor` for Cranelift; a `.clif` repro should come with a Wasm
repro showing the bug is reachable from Wasm), and "Write Good Bug Reports" in
`docs/contributing-fuzzing.md`. Also state whether the bug reproduces in
release mode, and deduplicate by root cause.

Patches should be minimal, fix the root cause rather than the symptom, and add
a regression test (usually a `.wast` file in `tests/misc_testsuite/` or a test
in `tests/all/`).

## Anything to leave alone

Code specific to the other Tier 1 platforms (macOS and Windows on x86-64) is
in scope but cannot be run here: report such issues only with a strong,
concrete argument from the code.
