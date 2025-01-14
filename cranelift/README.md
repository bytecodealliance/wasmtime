Cranelift Code Generator
========================

**A [Bytecode Alliance][BA] project**

[Website](https://cranelift.dev/)

Cranelift is a low-level retargetable code generator. It translates a
[target-independent intermediate representation](docs/ir.md)
into executable machine code.

[BA]: https://bytecodealliance.org/
[![Build Status](https://github.com/bytecodealliance/wasmtime/workflows/CI/badge.svg)](https://github.com/bytecodealliance/wasmtime/actions)
[![Chat](https://img.shields.io/badge/chat-zulip-brightgreen.svg)](https://bytecodealliance.zulipchat.com/#narrow/stream/217117-cranelift/topic/general)
![Minimum rustc 1.37](https://img.shields.io/badge/rustc-1.37+-green.svg)
[![Documentation Status](https://docs.rs/cranelift/badge.svg)](https://docs.rs/cranelift)

For more information, see [the documentation](docs/index.md).

For an example of how to use the JIT, see the [JIT Demo], which
implements a toy language.

[JIT Demo]: https://github.com/bytecodealliance/cranelift-jit-demo

For an example of how to use Cranelift to run WebAssembly code, see
[Wasmtime], which implements a standalone, embeddable, VM using Cranelift.

[Wasmtime]: https://github.com/bytecodealliance/wasmtime

Status
------

Cranelift currently supports enough functionality to run a wide variety
of programs, including all the functionality needed to execute
WebAssembly (MVP and various extensions like SIMD), although it needs to be
used within an external WebAssembly embedding such as Wasmtime to be part of a
complete WebAssembly implementation. It is also usable as a backend for
non-WebAssembly use cases: for example, there is an effort to build a [Rust
compiler backend] using Cranelift.

Cranelift is production-ready, and is used in production in several places, all
within the context of Wasmtime. It is carefully fuzzed as part of Wasmtime with
differential comparison against V8 and the executable Wasm spec, and the
register allocator is separately fuzzed with symbolic verification. There is an
active effort to formally verify Cranelift's instruction-selection backends. We
take security seriously and have a [security policy] as a part of Bytecode
Alliance.

Cranelift has four backends: x86-64, aarch64 (aka ARM64), s390x (aka IBM
Z) and riscv64. All backends fully support enough functionality for Wasm MVP, and
x86-64 and aarch64 fully support SIMD as well. On x86-64, Cranelift supports
both the System V AMD64 ABI calling convention used on many platforms and the
Windows x64 calling convention. On aarch64, Cranelift supports the standard
Linux calling convention and also has specific support for macOS (i.e., M1 /
Apple Silicon).

Cranelift's code quality is within range of competitiveness to browser JIT
engines' optimizing tiers. A [recent paper] includes third-party benchmarks of
Cranelift, driven by Wasmtime, against V8 and an LLVM-based Wasm engine, WAVM
(Fig 22).  The speed of Cranelift's generated code is ~2% slower than that of
V8 (TurboFan), and ~14% slower than WAVM (LLVM). Its compilation speed, in the
same paper, is measured as approximately an order of magnitude faster than WAVM
(LLVM). We continue to work to improve both measures.

[Rust compiler backend]: https://github.com/rust-lang/rustc_codegen_cranelift
[security policy]: https://bytecodealliance.org/security
[recent paper]: https://arxiv.org/abs/2011.13127

The core codegen crates have minimal dependencies and are carefully written to
handle malicious or arbitrary compiler input: in particular, they do not use
callstack recursion.

Cranelift performs some basic mitigations for Spectre attacks on heap bounds
checks, table bounds checks, and indirect branch bounds checks; see
[#1032] for more.

[#1032]: https://github.com/bytecodealliance/wasmtime/issues/1032

Cranelift's APIs are not yet considered stable, though we do follow
semantic-versioning (semver) with minor-version patch releases.

Cranelift generally requires the latest stable Rust to build as a policy, and
is tested as such, but we can incorporate fixes for compilation with older Rust
versions on a best-effort basis.

Contributing
------------

If you're interested in contributing to Cranelift: thank you! We have a
[contributing guide] which will help you getting involved in the Cranelift
project.

[contributing guide]: https://bytecodealliance.github.io/wasmtime/contributing.html

Planned uses
------------

Cranelift is designed to be a code generator for WebAssembly, but it is
general enough to be useful elsewhere too. The initial planned uses that
affected its design were:

- [Wasmtime non-Web wasm engine](https://github.com/bytecodealliance/wasmtime).
- [Debug build backend for the Rust compiler](rustc.md).
- WebAssembly compiler for the SpiderMonkey engine in Firefox
  (currently not planned anymore; SpiderMonkey team may re-assess in
  the future).
- Backend for the IonMonkey JavaScript JIT compiler in Firefox
  (currently not planned anymore; SpiderMonkey team may re-assess in
  the future).

Building Cranelift
------------------

Cranelift uses a [conventional Cargo build
process](https://doc.rust-lang.org/cargo/guide/working-on-an-existing-project.html).

Cranelift consists of a collection of crates, and uses a [Cargo
Workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html),
so for some cargo commands, such as `cargo test`, the `--all` is needed
to tell cargo to visit all of the crates.

`test-all.sh` at the top level is a script which runs all the cargo
tests and also performs code format, lint, and documentation checks.

<details>
<summary>Log configuration</summary>

Cranelift uses the `log` crate to log messages at various levels. It doesn't
specify any maximal logging level, so embedders can choose what it should be;
however, this can have an impact of Cranelift's code size. You can use `log`
features to reduce the maximum logging level. For instance if you want to limit
the level of logging to `warn` messages and above in release mode:

```
[dependency.log]
...
features = ["release_max_level_warn"]
```
</details>

Editor Support
--------------

Editor support for working with Cranelift IR (clif) files:

 - Vim: https://github.com/bytecodealliance/cranelift.vim
