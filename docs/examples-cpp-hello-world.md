# Hello, world!

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/hello.cc

This example shows off how to instantiate a simple wasm module with a host function import and call an exported function.

## `hello.wat`

```wat
{{#include ../examples/hello.wat}}
```

## `hello.cc`

```cpp
{{#include ../examples/hello.cc}}
```
