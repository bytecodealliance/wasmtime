# WASI (Preview 2)

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasi/main.rs

This example shows how to use the [`wasmtime-wasi`] crate to define WASI
functions within a [`Linker`] which can then be used to instantiate a
WebAssembly module.

[`wasmtime-wasi`]: https://crates.io/crates/wasmtime-wasi
[`Linker`]: https://docs.rs/wasmtime/*/wasmtime/struct.Linker.html

## WebAssembly Component Source Code

For this WASI example, this Hello World program is compiled to a WebAssembly module using the WASI Preview 2 (WASIp2) API.

`wasi.rs`
```rust
{{#include ../examples/wasi/wasm/wasi.rs}}
```

> Building instructions:
> 1. Have Rust installed
> 2. Add WASIp2 target if you haven't already: `rustup target add wasm32-wasip2`
> 3. `cargo build --target wasm32-wasip2`

Building this program generates `target/wasm32-wasip2/debug/wasi.wasm`, used below.

### Invoke the WASM component

This example shows adding and configuring the WASI imports to invoke the above WASM component.

`main.rs`
```rust,ignore
{{#include ../examples/wasi/main.rs}}
```

### Async example

This [async example code][code2] shows how to use the [wasmtime-wasi][`wasmtime-wasi`] module to
execute the same WASI Preview 2 WebAssembly component from the example above. This example requires the `wasmtime` crate `async` feature to be enabled.

This does not require any change to the WebAssembly module, it's just the WASI API host functions which are implemented to be async. See [wasmtime async support](https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.async_support).

[code2]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasi-async/main.rs
[`wasmtime-wasi`]: https://docs.rs/wasmtime-wasi/*/wasmtime_wasi/preview2/index.html

```rust,ignore
{{#include ../examples/wasi-async/main.rs}}
```

You can also [browse this source code online][code2] and clone the wasmtime
repository to run the example locally.

## Beyond Basics

Please see these references:
* The [book](https://component-model.bytecodealliance.org) for understanding the component model of WASIp2.
* [Bindgen Examples](https://docs.rs/wasmtime/latest/wasmtime/component/bindgen_examples/index.html) for implementing WASIp2 hosts and guests.