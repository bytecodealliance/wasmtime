# Serializing and Deserializing Modules

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/serialize.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/serialize.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/serialize.cc)

This example shows how to compile a module once and serialize its compiled representation to disk and later deserialize it to skip compilation on the critical path. See also the [pre-compilation example](examples-pre-compiling-wasm.md) for ahead-of-time compilation.

## Host Source

<!-- langtabs-start -->

```rust
{{#include ../examples/serialize.rs}}
```

```c
{{#include ../examples/serialize.c}}
```

```cpp
{{#include ../examples/serialize.cc}}
```

<!-- langtabs-end -->
