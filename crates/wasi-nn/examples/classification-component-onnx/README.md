# Onnx Backend Classification Component Example

This example demonstrates how to use the `wasi-nn` crate to run a classification using the
[ONNX Runtime](https://onnxruntime.ai/) backend from a WebAssembly component.

It supports CPU and GPU (Nvidia CUDA) execution targets.

**Note:**
GPU execution target only supports Nvidia CUDA (onnx-cuda) as execution provider (EP) for now.

## Build

In this directory, run the following command to build the WebAssembly component:
```console
# build component for target wasm32-wasip1
cargo component build

# build component for target wasm32-wasip2
cargo component build --target wasm32-wasip2
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
    ./crates/wasi-nn/examples/classification-component-onnx/target/wasm32-wasip2/debug/classification-component-onnx.wasm
```

#### GPU (CUDA) Execution:
```sh
# path to `libonnxruntime_providers_cuda.so` downloaded by `ort-sys`
export LD_LIBRARY_PATH={wasmtime_workspace}/target/debug

./target/debug/wasmtime run \
    -Snn \
    --dir ./crates/wasi-nn/examples/classification-component-onnx/fixture/::fixture \
    ./crates/wasi-nn/examples/classification-component-onnx/target/wasm32-wasip2/debug/classification-component-onnx.wasm \
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

To see trace logs from `wasmtime_wasi_nn` or `ort`, run Wasmtime with `WASMTIME_LOG` enabled, e.g.,

```sh
WASMTIME_LOG=wasmtime_wasi_nn=warn ./target/debug/wasmtime run ...
WASMTIME_LOG=ort=warn ./target/debug/wasmtime run ...
```

## Prerequisites for GPU(CUDA) Support
- NVIDIA GPU with CUDA support
- CUDA Toolkit 12.x with cuDNN 9.x
- Build wasmtime with `wasmtime-wasi-nn/onnx-cuda` feature

## ONNX Runtime's Fallback Behavior

If the GPU execution provider is requested (by passing `gpu`) but the device does not have a GPU or the necessary CUDA drivers are missing, ONNX Runtime will **silently fall back** to the CPU execution provider. The application will continue to run, but inference will happen on the CPU.

To verify if fallback is happening, you can enable ONNX Runtime logging:

1. Build Wasmtime with the additional `wasmtime-wasi-nn/ort-tracing` feature:
   ```sh
   cargo build --features component-model,wasi-nn,wasmtime-wasi-nn/onnx-cuda,wasmtime-wasi-nn/ort-tracing
   ```

2. Run Wasmtime with `WASMTIME_LOG` enabled to see `ort` warnings:
   ```sh
   WASMTIME_LOG=ort=warn ./target/debug/wasmtime run ...
   ```
   You should see a warning like: `No execution providers from session options registered successfully; may fall back to CPU.`
