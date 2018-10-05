# Fuzzing

This document describes how to fuzz cranelift with [`cargo-fuzz`]. The fuzz targets use `wasm-opt`
from [`binaryen-rs`] to generate valid WebAssembly modules from the fuzzed input supplied by
`cargo-fuzz` (via [libfuzzer]). In this scheme coverage feedback from both cranelift and the
`wasm-opt` input generation code is used to inform the fuzzer.

[`cargo-fuzz`]: https://github.com/rust-fuzz/cargo-fuzz
[`binaryen-rs`]: https://github.com/pepyakin/binaryen-rs
[libfuzzer]: http://llvm.org/docs/LibFuzzer.html

# Usage

1. Install all dependencies required to build `binaryen-rs` and `cargo-fuzz` (including `cmake`)
2. Use the rust nightly toolchain (required by `cargo-fuzz`): `rustup override set nightly`
3. Execute the fuzz target: `cargo fuzz run fuzz_translate_module`
