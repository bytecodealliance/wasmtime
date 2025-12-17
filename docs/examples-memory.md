# Using linear memory

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/memory.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/memory.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/memory.cc)

This example shows off how to interact with wasm memory in a module. Be sure to
read the documentation for [`Memory`] as well.

[`Memory`]: https://bytecodealliance.github.io/wasmtime/api/wasmtime/struct.Memory.html

## Wasm Source

```wat
{{#include ../examples/memory.wat}}
```

## Host Source

<!-- langtabs-start -->

```rust
{{#include ../examples/memory.rs}}
```

```c
{{#include ../examples/memory.c}}
```

```cpp
{{#include ../examples/memory.cc}}
```

<!-- langtabs-end -->
