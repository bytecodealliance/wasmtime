# Onnx Backend Classification Component Example

This example demonstrates how to use the `wasi-nn` crate to run a classification using the
[ONNX Runtime](https://onnxruntime.ai/) backend from a WebAssembly component.

It supports CPU and GPU (Nvidia CUDA) execution targets.

**Note:**
GPU execution target only supports Nvidia CUDA (onnx-cuda) as execution provider (EP) for now.

## Build

In this directory, run the following command to build the WebAssembly component:
```console
cargo component build
```

## Running the Example

In the Wasmtime root directory, run the following command to build the Wasmtime CLI and run the WebAssembly component:

### Building Wasmtime

#### For CPU-only execution:
```sh
cargo build --features component-model,wasi-nn,wasmtime-wasi-nn/onnx-download
```

#### For GPU (Nvidia CUDA) support:
```sh
cargo build --features component-model,wasi-nn,wasmtime-wasi-nn/onnx-cuda,wasmtime-wasi-nn/onnx-download
```

### Running with Different Execution Targets

The execution target is controlled by passing a single argument to the WASM module.

Arguments:
- No argument or `cpu` - Use CPU execution
- `gpu` or `cuda` - Use GPU/CUDA execution

#### CPU Execution (default):
```sh
./target/debug/wasmtime run \
    -Snn \
    --dir ./crates/wasi-nn/examples/classification-component-onnx/fixture/::fixture \
    ./crates/wasi-nn/examples/classification-component-onnx/target/wasm32-wasip1/debug/classification-component-onnx.wasm
```

#### GPU (CUDA) Execution:
```sh
# path to `libonnxruntime_providers_cuda.so` downloaded by `ort-sys`
export LD_LIBRARY_PATH={wasmtime_workspace}/target/debug

./target/debug/wasmtime run \
    -Snn \
    --dir ./crates/wasi-nn/examples/classification-component-onnx/fixture/::fixture \
    ./crates/wasi-nn/examples/classification-component-onnx/target/wasm32-wasip1/debug/classification-component-onnx.wasm \
    gpu

```

## Expected Output

You should get output similar to:
```txt
No execution target specified, defaulting to CPU
Read ONNX model, size in bytes: 4956208
Loaded graph into wasi-nn with Cpu target
Created wasi-nn execution context.
Read ONNX Labels, # of labels: 1000
Executed graph inference
Retrieved output data with length: 4000
Index: n02099601 golden retriever - Probability: 0.9948673
Index: n02088094 Afghan hound, Afghan - Probability: 0.002528982
Index: n02102318 cocker spaniel, English cocker spaniel, cocker - Probability: 0.0010986356
```

When using GPU target, the first line will indicate the selected execution target.
You can monitor GPU usage using cmd `watch -n 1 nvidia-smi`.

## Prerequisites for GPU(CUDA) Support
- NVIDIA GPU with CUDA support
- CUDA Toolkit 12.x with cuDNN 9.x
- Build wasmtime with `wasmtime-wasi-nn/onnx-cuda` feature
