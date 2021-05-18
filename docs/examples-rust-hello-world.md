# Hello, world!

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/hello.rs

This example shows off how to instantiate a simple wasm module and interact with
it. For more information about the types used here be sure to review the [core
concepts of the `wasmtime`
API](https://docs.rs/wasmtime/*/wasmtime/#core-concepts) as well as the general
[API documentation](https://docs.rs/wasmtime).

## `hello.wat`

```wat
{{#include ../examples/hello.wat}}
```


## `hello.rs`

```rust,ignore
{{#include ../examples/hello.rs}}
```
