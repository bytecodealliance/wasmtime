# wasmtime-wasi-nn

This crate enables support for the [wasi-nn] API in Wasmtime. Currently it
contains an implementation of [wasi-nn] using OpenVINO™ but in the future it
could support multiple machine learning backends. Since the [wasi-nn] API is
expected to be an optional feature of WASI, this crate is currently separate
from the [wasi-common] crate. This crate is experimental and its API,
functionality, and location could quickly change.

[examples]: examples
[openvino]: https://crates.io/crates/openvino
[wasi-nn]: https://github.com/WebAssembly/wasi-nn
[wasi-common]: ../wasi-common
[bindings]: https://crates.io/crates/wasi-nn

### Use

Use the Wasmtime APIs to instantiate a Wasm module and link in the `wasi-nn`
implementation as follows:

```rust
let wasi_nn = WasiNnCtx::new()?;
wasmtime_wasi_nn::witx::add_to_linker(...);
```

### Build

```sh
$ cargo build
```

To use the WIT-based ABI, compile with `--features component-model` and use `wasmtime_wasi_nn::wit::add_to_linker`.

### Example

An end-to-end example demonstrating ML classification is included in [examples]:
`examples/classification-example` contains a standalone Rust project that uses
the [wasi-nn] APIs and is compiled to the `wasm32-wasip1` target using the
high-level `wasi-nn` [bindings].
