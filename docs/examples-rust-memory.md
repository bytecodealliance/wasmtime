# Using linear memory

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/memory.rs

This example shows off how to interact with wasm memory in a module. Be sure to
read the documentation for [`Memory`] as well.

[`Memory`]: https://bytecodealliance.github.io/wasmtime/api/wasmtime/struct.Memory.html

## `memory.wat`

```wat
{{#include ../examples/memory.wat}}
```


## `memory.rs`

```rust,ignore
{{#include ../examples/memory.rs}}
```
