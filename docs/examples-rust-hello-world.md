# Hello, world!

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/hello.rs

This example shows off how to instantiate a simple wasm module and interact with
it.

## `hello.wat`

```wat
{{#include ../examples/hello.wat}}
```


## `hello.rs`

```rust,ignore
{{#include ../examples/hello.rs}}
```
