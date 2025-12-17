# Linking modules

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/linking.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/linking.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/linking.cc)

This example shows off how to compile and instantiate modules which link
together. Be sure to read the API documentation for [`Linker`] as well.

[`Linker`]: https://docs.rs/wasmtime/0.26.0/wasmtime/struct.Linker.html

## Wasm: `linking1.wat`

```wat
{{#include ../examples/linking1.wat}}
```

## Wasm: `linking2.wat`

```wat
{{#include ../examples/linking2.wat}}
```

## Host Source

<!-- langtabs-start -->

```rust
{{#include ../examples/linking.rs}}
```

```c
{{#include ../examples/linking.c}}
```

```cpp
{{#include ../examples/linking.cc}}
```

<!-- langtabs-end -->
