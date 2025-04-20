# Image Classification Example

This example project demonstrates using the `wasi-nn` API to perform machine
learning (ML) inference. It shows how to collect and use the various parts of a
wasi-nn program:
- an ML __framework__ ("backend" in wasi-nn terms)
- an ML __model__ ("graph" terms in wasi-nn terms)
- a WebAssembly __program__
- a wasi-nn-compatible __engine__

### Pre-requisite: Framework

This example uses the OpenVINO framework: [installation
instructions][openvino-install]. If you're interested in how the engine forwards
calls from the WebAssembly program to this framework, see the
[backend][openvino-backend] source code.

[openvino-install]: https://docs.openvino.ai/2025/get-started/install-openvino.html
[openvino-backend]: ../../src/backend/openvino.rs

### Pre-requisite: Model

MobileNet is a small, common model for classifying images; it returns the
probabilities for words that best describe the image. To retrieve the files
needed to use this model on OpenVINO, download:

```
wget https://download.01.org/openvinotoolkit/fixtures/mobilenet/mobilenet.bin -O fixture/model.bin
wget https://download.01.org/openvinotoolkit/fixtures/mobilenet/mobilenet.xml -O fixture/model.xml
wget https://download.01.org/openvinotoolkit/fixtures/mobilenet/tensor-1x224x224x3-f32.bgr -O fixture/tensor.bgr
```

The `.bgr` file is a tensor representation of an image file (more details
[here]).

[here]: https://download.01.org/openvinotoolkit/fixtures/mobilenet

### Pre-requisite: Program

Compile this Rust [example] to a WebAssembly program using the `wasm32-wasip1`
target. This requires a Rust toolchain (e.g., [rustup]) and the appropriate
compilation target (e.g., `rustup target add wasm32-wasip1`). To compile the
program to a `*.wasm` file in the `target` directory:

```
cargo build --target=wasm32-wasip1
```

[example]: src/main.rs
[rustup]: https://rustup.rs

### Pre-requisites: Engine

This example uses Wasmtime, which contains a wasi-nn [implementation][crate]. To
use Wasmtime, follow the instructions to either [build] or [install] it.

[build]: https://docs.wasmtime.dev/contributing-building.html
[crate]: ../..
[install]: https://docs.wasmtime.dev/cli-install.html

### Run

With the pre-requisites in place, run the example:

```
<path>/<to>/wasmtime run --wasi=nn --dir=fixture target/wasm32-wasip1/debug/wasi-nn-example.wasm
```

Some words of explanation: the `--wasi` flag enables the wasi-nn proposal (see
`-S help`), the `--dir` maps our host-side `fixture` directory to a directory of
the same name in the guest, and we pass the `*.wasm` module as the sole
argument. For this model (see the [source][example]), we expect to see the list
of tags that mostly likely describe the image:

```
...
Found results, sorted top 5: [InferenceResult(885, 0.3958254), InferenceResult(904, 0.36464655), InferenceResult(84, 0.010480323), InferenceResult(911, 0.0082290955), InferenceResult(741, 0.007244849)]
```
