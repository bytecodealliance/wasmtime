# Working with `externref`

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/externref.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/externref.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/externref.cc)

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/externref.rs

This example shows how to pass opaque host references into and out of WebAssembly using `externref`.

## Wasm Source

```wat
{{#include ../examples/externref.wat}}
```

## Host Source

<!-- langtabs-start -->

```rust
{{#include ../examples/externref.rs}}
```

```c
{{#include ../examples/externref.c}}
```

```cpp
{{#include ../examples/externref.cc}}
```

<!-- langtabs-end -->
