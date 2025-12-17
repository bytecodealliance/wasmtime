# Working with Multiple Memories

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/multimemory.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/multimemory.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/multimemory.cc)

This example demonstrates using the multiple memories proposal, instantiating a module that imports and exports more than one linear memory.

## Wasm Source

```wat
{{#include ../examples/multimemory.wat}}
```

## Host Source

<!-- langtabs-start -->

```rust
{{#include ../examples/multimemory.rs}}
```

```c
{{#include ../examples/multimemory.c}}
```

```cpp
{{#include ../examples/multimemory.cc}}
```

<!-- langtabs-end -->
