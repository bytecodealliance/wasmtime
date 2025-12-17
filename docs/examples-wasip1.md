# WASI

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasip1/main.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasip1/main.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasip1/main.cc)

This example shows off how to instantiate a wasm module using WASI imports.

## Wasm Source

```rust,ignore
{{#include ../examples/wasm/wasi.rs}}
```

## Host Source

<!-- langtabs-start -->

```rust
{{#include ../examples/wasip1/main.rs}}
```

```c
{{#include ../examples/wasip1/main.c}}
```

```cpp
{{#include ../examples/wasip1/main.cc}}
```

<!-- langtabs-end -->
