# Multi-threaded Embeddings

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/threads.rs

This example demonstrates using Wasmtime in multi-threaded runtimes.

## `threads.wat`

```wat
{{#include ../examples/threads.wat}}
```

## `threads.rs`

```rust,ignore
{{#include ../examples/threads.rs}}
```
