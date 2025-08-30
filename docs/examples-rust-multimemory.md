# Working with Multiple Memories

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/multimemory.rs

This example demonstrates using the multiple memories proposal, instantiating a module that imports and exports more than one linear memory.

## `multimemory.wat`

```wat
{{#include ../examples/multimemory.wat}}
```

## `multimemory.rs`

```rust,ignore
{{#include ../examples/multimemory.rs}}
```
