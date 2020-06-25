# Embedding in C

This section is intended to showcase the C embedding API for Wasmtime. The C
embedding API is based on the [proposed wasm C embedding API][proposal] (namely
[`wasm.h`]) and has a few extension headers (like [`wasi.h`] and
[`wasmtime.h`]) which are intended to eventually become part of the standard
themselves one day.

[proposal]: https://github.com/webassembly/wasm-c-api
[`wasm.h`]: https://github.com/WebAssembly/wasm-c-api/blob/master/include/wasm.h
[`wasi.h`]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/include/wasi.h
[`wasmtime.h`]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/include/wasmtime.h
