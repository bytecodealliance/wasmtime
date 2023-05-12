# Debugging

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/fib-debug/main.c

This example shows off how to set up a module for dynamic runtime debugging via
a native debugger like GDB or LLDB.

## `main.c`

```c
{{#include ../examples/fib-debug/main.c}}
```
