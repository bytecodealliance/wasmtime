# Multithreaded Embedding

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/threads.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/threads.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/threads.cc)

This example demonstrates using Wasmtime in multithreaded runtimes.

## Wasm Source

```wat
{{#include ../examples/threads.wat}}
```

## Host Source

<!-- langtabs-start -->

```rust
{{#include ../examples/threads.rs}}
```

```c
{{#include ../examples/threads.c}}
```

```cpp
{{#include ../examples/threads.cc}}
```

<!-- langtabs-end -->
