# Named Model Example

This example is mostly the same as the [`classification-example`] but uses
wasi-nn's "named model" API to load the ML model before the WebAssembly program
executes. Instead of loading the model bytes during program execution (which may
be prohibitively slow for large models), this wasi-nn extension allows the model
to be loaded by the engine prior to execution.

### Pre-requisites

The example pre-requisites are mostly the same as that of the
[`classification-example`], except that we separate the model files that the
engine pre-loads (`mobilenet/*`) from the image file read via WASI from the host
(`fixture/*`):

```
wget https://download.01.org/openvinotoolkit/fixtures/mobilenet/mobilenet.bin -O mobilenet/model.bin
wget https://download.01.org/openvinotoolkit/fixtures/mobilenet/mobilenet.xml -O mobilenet/model.xml
wget https://download.01.org/openvinotoolkit/fixtures/mobilenet/tensor-1x224x224x3-f32.bgr -O fixture/tensor.bgr
```

As before, this Rust [example] compiles to a WebAssembly program using the
`wasm32-wasip1` target: `cargo build --target=wasm32-wasip1`.

[example]: src/main.rs

### Run

The program is invoked with slightly different flags than the
[`classification-example`]:

```
<path>/<to>/wasmtime run --dir=fixture --wasi=nn --wasi=nn-graph=openvino::mobilenet target/wasm32-wasip1/debug/wasi-nn-example-named.wasm
```

The primary difference is the addition of `--wasi=nn-graph=openvino::mobilenet`:
this informs Wasmtime's wasi-nn implementation to pre-load a named model from
the `mobilenet` directory using the `openvino` encoding. Note that, in this
implementation, the model name (i.e., `mobilenet`) is derived from the base name
of the path passed in `--nn-graph=<encoding>::<path>`.

[`classification-example`]: ../classification-example
