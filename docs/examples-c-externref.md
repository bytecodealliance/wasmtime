# Working with `externref`

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/externref.c

This example shows how to pass opaque host references into and out of WebAssembly using `externref`.

## `externref.wat`

```wat
{{#include ../examples/externref.wat}}
```

## `externref.c`

```c
{{#include ../examples/externref.c}}
```
