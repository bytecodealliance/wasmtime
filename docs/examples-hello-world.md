# Hello, world!

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/hello.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/hello.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/hello.cc)

This example shows off how to instantiate a simple wasm module and interact with
it.

## Wasm Source

```wat
{{#include ../examples/hello.wat}}
```

## Host Source

<!-- langtabs-start -->
```rust
{{#include ../examples/hello.rs}}
```

```c
{{#include ../examples/hello.c}}
```

```cpp
{{#include ../examples/hello.cc}}
```
<!-- langtabs-end -->
