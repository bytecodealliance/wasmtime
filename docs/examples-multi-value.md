# Using multi-value

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/multi.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/multi.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/multi.cc)

This example shows off how to interact with a wasm module that uses multi-value
exports and imports.

## Wasm Source

```wat
{{#include ../examples/multi.wat}}
```

## Host Source

<!-- langtabs-start -->

```rust
{{#include ../examples/multi.rs}}
```

```c
{{#include ../examples/multi.c}}
```

```cpp
{{#include ../examples/multi.cc}}
```

<!-- langtabs-end -->
