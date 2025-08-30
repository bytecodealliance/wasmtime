# Working with Multiple Memories

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/multimemory.c

This example demonstrates instantiating and interacting with a module that has multiple linear memories.

## `multimemory.wat`

```wat
{{#include ../examples/multimemory.wat}}
```

## `multimemory.c`

```c
{{#include ../examples/multimemory.c}}
```
