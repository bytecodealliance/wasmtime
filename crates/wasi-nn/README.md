# wasmtime-wasi-nn

This crate enables support for the [wasi-nn] API in Wasmtime. Currently it contains an implementation of [wasi-nn] using
OpenVINO™ but in the future it could support multiple machine learning backends. Since the [wasi-nn] API is expected
to be an optional feature of WASI, this crate is currently separate from the [wasi-common] crate. This crate is
experimental and its API, functionality, and location could quickly change.

[examples]: examples
[openvino]: https://crates.io/crates/openvino
[wasi-nn]: https://github.com/WebAssembly/wasi-nn
[wasi-common]: ../wasi-common

### Use

Use the Wasmtime APIs to instantiate a Wasm module and link in the `WasiNn` implementation as follows:

```
let wasi_nn = WasiNn::new(&store, WasiNnCtx::new()?);
wasi_nn.add_to_linker(&mut linker)?;
```

### Build

This crate should build as usual (i.e. `cargo build`) but note that using an existing installation of OpenVINO™, rather
than building from source, will drastically improve the build times. See the [openvino] crate for more information

### Example

An end-to-end example demonstrating ML classification is included in [examples]:
 - `tests/wasi-nn-rust-bindings` contains ergonomic bindings for writing Rust code against the [wasi-nn] APIs
 - `tests/classification-example` contains a standalone Rust project that uses the [wasi-nn] APIs and is compiled to the 
 `wasm32-wasi` target using the `wasi-nn-rust-bindings`

Run the example from the Wasmtime project directory:

```
ci/run-wasi-nn-example.sh
```
