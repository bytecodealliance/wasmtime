# Asynchronous Host Functions

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/async.cc

This example demonstrates configuring Wasmtime for asynchronous operation and calling async host functions from wasm.

## Wasm Source

```wat
{{#include ../examples/async.wat}}
```

## Host Source

<!-- langtabs-start -->

```cpp
{{#include ../examples/async.cc}}
```

<!-- langtabs-end -->
