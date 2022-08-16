# wasmtime-wasi-nn

This crate enables support for the [wasi-nn] API in Wasmtime. Currently it contains an implementation of [wasi-nn] for both
OpenVINO™ and TensorFlow, but in the future it could support additional machine learning backends. Since the [wasi-nn] API is expected
to be an optional feature of WASI, this crate is currently separate from the [wasi-common] crate. This crate is
experimental and its API, functionality, and location could quickly change.

[examples]: examples
[openvino]: https://crates.io/crates/openvino
[tensorflow]: https://crates.io/crates/tensorflow
[wasi-nn]: https://github.com/WebAssembly/wasi-nn
[wasi-common]: ../wasi-common

### Use

Use the Wasmtime APIs to instantiate a Wasm module and link in the `WasiNn` implementation as follows:

```
let wasi_nn = WasiNn::new(&store, WasiNnCtx::new()?);
wasi_nn.add_to_linker(&mut linker)?;
```

### Build

This crate should build as usual (i.e. `cargo build`) but note that using an
existing installation of OpenVINO™ or TensorFlow, rather than building from
source, will drastically improve the build times. See the [openvino] and
[tensorflow] crates for more information.

### Example

An end-to-end example demonstrating ML classification is included in [examples]:
 - `examples/openvino` contains a standalone Rust project that uses the
    [wasi-nn] APIs to use OpenVINO models for inference
 - `examples/tensorflow` is the same as `examples/openvino` but for the
   Tensorflow backend

Run the example from the Wasmtime project directory:

```
ci/run-wasi-nn-openvino-example.sh
```

### Additional Examples

You can find more in depth examples using both the OpenVINO™ and TensorFlow backends in the [wasi-nn bindings] git repository. There's one for [Rust] as well as [AssemblyScript].

[wasi-nn bindings]: https://github.com/bytecodealliance/wasi-nn
[Rust]: https://github.com/bytecodealliance/wasi-nn/tree/main/rust/examples/classification-example
[AssemblyScript]: https://github.com/bytecodealliance/wasi-nn/tree/main/assemblyscript
