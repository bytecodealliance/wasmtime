# Cranelift Documentation

## Miscellaneous documentation pages:

 - [Cranelift IR](ir.md)
   Cranelift IR is the data structure that most of the compiler operates on.

 - [Testing Cranelift](testing.md)
   This page documents Cranelift's testing frameworks.

 - [Cranelift compared to LLVM](compare-llvm.md)
   LLVM and Cranelift have similarities and differences.

## Cranelift crate documentation:

 - [cranelift](https://docs.rs/cranelift)
    This is an umbrella crate that re-exports the codegen and frontend crates,
    to make them easier to use.

 - [cranelift-codegen](https://docs.rs/cranelift-codegen)
    This is the core code generator crate. It takes Cranelift IR as input
    and emits encoded machine instructions, along with symbolic relocations,
    as output.

 - [cranelift-codegen-meta](https://docs.rs/cranelift-codegen-meta)
    This crate contains the meta-language utilities and descriptions used by the
    code generator.

 - [cranelift-wasm](https://docs.rs/cranelift-wasm)
    This crate translates WebAssembly code into Cranelift IR.

 - [cranelift-frontend](https://docs.rs/cranelift-frontend)
    This crate provides utilities for translating code into Cranelift IR.

 - [cranelift-native](https://docs.rs/cranelift-native)
    This crate performs auto-detection of the host, allowing Cranelift to
    generate code optimized for the machine it's running on.

 - [cranelift-reader](https://docs.rs/cranelift-reader)
    This crate translates from Cranelift IR's text format into Cranelift IR
    in in-memory data structures.

 - [cranelift-module](https://docs.rs/cranelift-module)
    This crate manages compiling multiple functions and data objects
    together.

 - [cranelift-object](https://docs.rs/cranelift-object)
    This crate provides a object-based backend for `cranelift-module`, which
    emits native object files using the
    `object <https://github.com/gimli-rs/object>`_ library.

 - [cranelift-jit](https://docs.rs/cranelift-jit)
    This crate provides a JIT backend for `cranelift-module`, which
    emits code and data into memory.
