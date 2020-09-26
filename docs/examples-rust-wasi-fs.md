# WASI

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasi/main.rs

This example shows off how to run a wasi binary with a memory filesystem.

## Wasm Source code

```rust,ignore
{{#include ../examples/wasi-fs/wasm/wasi-fs.rs}}
```


## `wasi-fs.rs`

```rust,ignore
{{#include ../examples/wasi-fs/main.rs}}
```
