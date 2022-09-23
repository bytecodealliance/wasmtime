# wasmtime-wasi-parallel

This crate enables experimental support for the [wasi-parallel] API in Wasmtime.

> __WARNING__: _this implementation is highly experimental, subject to change,
> and abuses the Wasmtime API!_ It is published as a proof-of-concept to discuss
> the design of the wasi-parallel specification.

Please open any issues related to this implementation in the [wasi-parallel]
repository. Feedback is appreciated!

The main idea is to expose a "parallel for" mechanism using WASI (see the
[explainer] for more details). The "parallel for" call is not limited to CPU
execution; this proof-of-concept implementation can execute parallel code on
both the CPU (using Wasmtime's JIT compiled functions) and eventually the GPU
(using OpenCL). If you plan to experiment with this crate, see the "Use" section
below.

[wasi-parallel]: https://github.com/WebAssembly/wasi-parallel
[explainer]: https://github.com/WebAssembly/wasi-parallel#readme

### Build

```
cargo build
```

### Test

```
cargo test
```

Note: the Rust code in `tests/rust` is compiled by `build.rs` to `tests/wasm`.

### Benchmark

```
cargo bench
```

### Use

When compiled with the `wasi-parallel` feature, this crate is usable from the
Wasmtime CLI:

```console
$ cargo build --features wasi-parallel
$ .../wasmtime run --wasi-modules experimental-wasi-parallel <module>
```
