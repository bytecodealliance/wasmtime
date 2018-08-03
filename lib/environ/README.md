This is the `wasmtime-environ` crate, which contains the implementations
of the `ModuleEnvironment` and `FuncEnvironment` traits from
[`cranelift-wasm`](https://crates.io/crates/cranelift-wasm). They effectively
implement an ABI for basic wasm compilation, which can be used for JITing,
native object files, or other purposes.
