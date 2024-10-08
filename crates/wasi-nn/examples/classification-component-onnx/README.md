# Onnx Backend Classification Component Example

This example demonstrates how to use the `wasi-nn` crate to run a classification using the
[ONNX Runtime](https://onnxruntime.ai/) backend from a WebAssembly component.

## Build
In this directory, run the following command to build the WebAssembly component:
```shell
cargo component build
```

In the Wasmtime root directory, run the following command to build the Wasmtime CLI and run the WebAssembly component:
```shell
# build wasmtime with component-model and WASI-NN with ONNX runtime support
cargo build --features component-model,wasi-nn,wasmtime-wasi-nn/onnx

# run the component with wasmtime
./target/debug/wasmtime run -Snn --dir ./crates/wasi-nn/examples/classification-component-onnx/fixture/::fixture ./crates/wasi-nn/examples/classification-component-onnx/target/wasm32-wasip1/debug/classification-component-onnx.wasm
```

You should get the following output:
```txt
Read ONNX model, size in bytes: 4956208
Loaded graph into wasi-nn
Created wasi-nn execution context.
Read ONNX Labels, # of labels: 1000
Set input tensor
Executed graph inference
Getting inferencing output
Retrieved output data with length: 4000
Index: n02099601 golden retriever - Probability: 0.9948673
Index: n02088094 Afghan hound, Afghan - Probability: 0.002528982
Index: n02102318 cocker spaniel, English cocker spaniel, cocker - Probability: 0.0010986356
```
