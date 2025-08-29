# Working with `externref`

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/externref.cc

This example shows how to pass opaque host references into and out of WebAssembly using `externref`.

## `externref.wat`

```wat
{{#include ../examples/externref.wat}}
```

## `externref.cc`

```cpp
{{#include ../examples/externref.cc}}
```
