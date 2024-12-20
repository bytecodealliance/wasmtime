# WASI

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasi/main.c

This example shows off how to instantiate a wasm module using WASI imports.

## Wasm Source code

```rust,ignore
{{#include ../examples/wasm/wasi.rs}}
```


## `wasi.c`

```c
{{#include ../examples/wasi/main.c}}
```
