# WASI

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/master/examples/wasi/main.rs

This example shows off how to instantiate a wasm module using WASI imports.

## Wasm Source code

```rust
{{#include ../examples/wasi/wasm/wasi.rs}}
```


## `wasi.rs`

```rust,ignore
{{#include ../examples/wasi/main.rs}}
```
