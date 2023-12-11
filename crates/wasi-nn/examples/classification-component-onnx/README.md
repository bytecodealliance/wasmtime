# Onnx Backend Classification Component Example

This example demonstrates how to use the `wasi-nn` crate to run a classification using the 
[ONNX Runtime](https://onnxruntime.ai/) backend from a WebAssembly component.

## Build
In this directory, run the following command to build the WebAssembly component:
```shell
cargo component build
```

In the wasmtime root directory, run the following command to build the wasmtime CLI and run the WebAssembly component:
```shell
cargo build --features component-model,wasi-nn

./target/debug/wasmtime run \ 
  --wasm-features component-model \ 
  --wasi-modules=experimental-wasi-nn \
  --mapdir fixture::./crates/wasi-nn/examples/classification-component-onnx/fixture \
  ./crates/wasi-nn/examples/classification-component-onnx/target/wasm32-wasi/debug/classification-component-onnx.wasm
```