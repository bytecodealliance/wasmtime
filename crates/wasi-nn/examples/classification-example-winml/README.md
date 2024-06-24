This example project demonstrates using the `wasi-nn` API to perform WinML-based
inference. We first build Wasmtime, a fast and secure runtime for WebAssembly,
and then build a WebAssembly example, which:
- reads an input image from [`fixture/kitten.png`],
- converts it to the correct tensor format,
- and then classifies the image using [`fixture/mobilenet.onnx`]

[`fixture/kitten.png`]: fixture/kitten.png
[`fixture/mobilenet.onnx`]: fixture/mobilenet.onnx
[`src/main.rs`]: src/main.rs
[build guide]: https://docs.wasmtime.dev/contributing-building.html

To run this example, perform the following steps on Windows 10 v1803 and later:

1. Build Wasmtime according to the [build guide], but enable the `winml`
   feature:
   ```shell
   cargo build --release --features wasmtime-wasi-nn/winml
   ```
1. Navigate to this directory from Wasmtime's top-level directory (referred to
   later as `%PROJECT_DIR%).
    ```
    set PROJECT_DIR=%CD%
    cd crates\wasi-nn\examples\classification-example-winml
    ```
1. Install the `wasm32-wasip1` Rust target:
    ```
    rustup target add wasm32-wasip1
    ```
1. Compile this example; the `wasm32-wasip1` output is a WebAssembly file:
    ```
    cargo build --release --target=wasm32-wasip1
    ```
1. Run the sample; the fixture directory containing the model and image must be
   mapped in to be accessible to WebAssembly.
    ```
    %PROJECT_DIR%\target\release\wasmtime.exe --dir fixture::fixture -S nn target\wasm32-wasip1\release\wasi-nn-example-winml.wasm
    ```
1. The example will print the top 5 classification results. To run with a
   different image or ONNX model, modify the files in the `fixture` directory
   along with any path changes this may cause [`src/main.rs`].
