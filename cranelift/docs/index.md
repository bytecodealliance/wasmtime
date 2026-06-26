# Cranelift Documentation

## Reference

 - [Cranelift IR](ir.md)
   The data structure that most of the compiler operates on: opcodes, types,
   basic blocks, function signatures, and the textual `.clif` format.

 - [Testing Cranelift](testing.md)
   Cranelift's testing frameworks: Rust unit tests, filecheck-based `.clif`
   file tests, and the available `test` commands.

 - [Cranelift compared to LLVM](compare-llvm.md)
   Similarities and differences between Cranelift and LLVM in terms of IR
   design, optimization model, and code generation approach.

 - [How ISLE is Integrated with Cranelift](isle-integration.md)
   How the ISLE DSL fits into Cranelift's build system and lowering pipeline.

## Architecture

 - [Backend Architecture](backend-architecture.md)
   How the backend pipeline works: CLIF → VCode → binary. Covers ISLE
   instruction selection, the `MachInst` trait, `VCode`, register allocation
   via regalloc2, and `MachBuffer` emission.

## Contributor Guides

 - [How to Add a New Backend](add-new-backend.md)
   Step-by-step guide to creating a new ISA backend: directory layout,
   register definitions, machine instruction enum, ABI, ISLE lowering rules,
   and binary emission.

 - [How to Add a New Instruction to CLIF](add-clif-instruction.md)
   Adding a new opcode to the Cranelift IR: format definitions, the meta
   DSL, verifier checks, interpreter support, and testing.

 - [How to Add a Machine Instruction and Lowering](add-machine-instruction.md)
   Adding a new machine instruction variant to a backend and writing ISLE
   rules to lower CLIF opcodes to it.

 - [Debugging Code Generation](debugging-codegen.md)
   Strategies for debugging miscompilations, panics, and register allocation
   failures: printing IR, minimizing test cases, diffing output, chaos mode,
   and common error patterns.

## Cranelift crate documentation

 - [cranelift](https://docs.rs/cranelift)
    Umbrella crate that re-exports `cranelift-codegen` and `cranelift-frontend`.

 - [cranelift-codegen](https://docs.rs/cranelift-codegen)
    Core code generator crate. Takes Cranelift IR as input and emits encoded
    machine instructions with symbolic relocations.

 - [cranelift-frontend](https://docs.rs/cranelift-frontend)
    Utilities for translating code into Cranelift IR, including SSA construction.

 - [cranelift-native](https://docs.rs/cranelift-native)
    Auto-detection of the host ISA and feature flags.

 - [cranelift-reader](https://docs.rs/cranelift-reader)
    Parses the textual `.clif` format into in-memory IR data structures.

 - [cranelift-module](https://docs.rs/cranelift-module)
    Manages compiling multiple functions and data objects together and linking
    them into a module.

 - [cranelift-object](https://docs.rs/cranelift-object)
    Object-file backend for `cranelift-module`, emitting native object files
    via the [object](https://github.com/gimli-rs/object) library.

 - [cranelift-jit](https://docs.rs/cranelift-jit)
    JIT backend for `cranelift-module`, emitting code and data directly into
    executable memory.
