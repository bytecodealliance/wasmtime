# Example of garbage collector

This example shows off how to implement a tracing garbage collector using stack
maps in [Cranelift](https://crates.io/crates/cranelift). The garbage collector
is a very simple implementation using Rust's built-in
[`std::alloc`](https://doc.rust-lang.org/std/alloc/index.html) allocator, uses global
state and does not support multi-threaded usage.

For a more detailed explanation of stack maps, see [Stack maps] and [New Stack Maps for Wasmtime and Cranelift].

[Stack maps]: /cranelift/docs/stack-maps.md
[New Stack Maps for Wasmtime and Cranelift]: https://bytecodealliance.org/articles/new-stack-maps-for-wasmtime#background-garbage-collection-safepoints-and-stack-maps

This sample current supports `x86`, `x86_64` and `aarch64`.
