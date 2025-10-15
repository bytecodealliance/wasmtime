# Working with `anyref`

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/anyref.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/anyref.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/anyref.cc)

This example demonstrates using `anyref` values.

## Wasm Source

```wat
{{#include ../examples/anyref.wat}}
```

## Host Source

<!-- langtabs-start -->

```rust
{{#include ../examples/anyref.rs}}
```

```c
{{#include ../examples/anyref.c}}
```

```cpp
{{#include ../examples/anyref.cc}}
```

<!-- langtabs-end -->
