<div align="center">
  <h1>Winch</h1>

  <h3>WebAssembly Intentionally Non-optimizing Compiler and Host</h3>

  <p>
    <strong>A WebAssembly baseline compiler</strong>
  </p>

  <strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>

  <p>
    <a href="https://github.com/bytecodealliance/wasmtime/actions?query=workflow%3ACI"><img src="https://github.com/bytecodealliance/wasmtime/workflows/CI/badge.svg" alt="build status" /></a>
    <a href="https://bytecodealliance.zulipchat.com/#narrow/stream/417703-winch"><img src="https://img.shields.io/badge/zulip-join_chat-brightgreen.svg" alt="zulip chat" /></a>
    <img src="https://img.shields.io/badge/rustc-stable+-green.svg" alt="supported rustc stable" />
    <a href="https://docs.rs/winch-codegen"><img src="https://docs.rs/winch-codegen/badge.svg" alt="Documentation Status" /></a>
  </p>
</div>

## About

Winch is a WebAssembly "baseline" or single-pass compiler designed for Wasmtime.

Winch's primary goal is compilation performance, therefore only certain, very
limited peephole optimations are applied.

For more details on the original motivation and goals, refer to the [Bytecode
Alliance RFC for Baseline Compilation in Wasmtime.][rfc].

[rfc]: https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-baseline-compilation.md

## Design principles

* Single pass over Wasm bytecode

* Function as the unit of compilation

* Machine code generation directly from Wasm bytecode – no intermediate
  representation

* Avoid reinventing machine-code emission – use Cranelift's instruction emitter
  code to create an assembler library

* Prioritize compilation performance over runtime performance

* Simple to verify by looking. It should be evident which machine instructions
  are emitted per WebAssembly operator

* Adding and iterating on new (WebAssembly and developer-facing) features should
  be simpler than doing it in an optimizing tier (Cranelift)


## Status

Winch's aim is to support all the backends officially supported by Wasmtime:

* x86\_64
* arm64
* riscv64
* s390x

The x86\_64 backend offers an almost-complete implementation, it currently
supports all the instructions that are part of WebAssembly's MVP, plus some of
the [feature extensions](feature-extensions). Refer to the [Tiers of
Support](tiers-of-support) for more details.


[feature-extensions]: https://webassembly.org/features/
[tiers-of-support]: https://docs.wasmtime.dev/stability-tiers.html
