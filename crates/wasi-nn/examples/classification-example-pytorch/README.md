This example project demonstrates using the `wasi-nn` API to perform PyTorch based inference. It consists of Rust code that is built using the `wasm32-wasip1` target.

To run this example: 
1. Ensure you set appropriate Libtorch enviornment variables according to [tch-rs instructions]( https://github.com/LaurentMazare/tch-rs?tab=readme-ov-file#libtorch-manual-install). 
    - Requires the C++ PyTorch library (libtorch) in version *v2.4.0* to be available on
your system. 
    - `export LIBTORCH=/path/to/libtorch`
2. Build Wasmtime  with `wasmtime-wasi-nn/pytorch` feature.
3. Navigate to this example directory `crates/wasi-nn/examples/classification-example-pytorch`.
4. Build this example `cargo build --target=wasm32-wasip1`.
5. Run the generated wasm file with wasmtime after mapping the directory containing Resnet18 `model.pt` and sample image `kitten.png`
    ```
    ${Wasmtime_root_dir}/target/debug/wasmtime -S nn --dir ${Wasmtime_root_dir}/crates/wasi-nn/examples/classification-example-pytorch::. ${Wasmtime_root_dir}/crates/wasi-nn/examples/classification-example-pytorch/target/wasm32-wasip1/debug/wasi-nn-example-pytorch.wasm
    ```
6. Check that result `281` has highest probability, which corresponds to `tabby cat`.