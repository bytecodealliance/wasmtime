# Linking modules together

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/linking.cc

This example shows off how to link multiple wasm modules together, where one module imports functions exported by another.

## `linking1.wat`

```wat
{{#include ../examples/linking1.wat}}
```

## `linking2.wat`

```wat
{{#include ../examples/linking2.wat}}
```

## `linking.cc`

```cpp
{{#include ../examples/linking.cc}}
```
