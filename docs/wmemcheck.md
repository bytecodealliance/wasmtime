# Wasm memcheck (wmemcheck)

wmemcheck provides the ability to check for invalid mallocs, reads, and writes
inside a Wasm module, as long as Wasmtime is able to make certain assumptions
(`malloc` and `free` functions are visible and your program uses only the
default allocator). This is analogous to the Valgrind tool's memory checker
(memcheck) tool for native programs.

How to use:

1. When building Wasmtime, add the CLI flag "--features wmemcheck" to compile with wmemcheck configured.
    > cargo build --features wmemcheck
2. When running your wasm module, add the CLI flag "-W wmemcheck".
    > wasmtime run -W wmemcheck test.wasm

If your program executes an invalid operation (load or store to non-allocated
address, double-free, or an internal error in malloc that allocates the same
memory twice) you will see an error that looks like a Wasm trap. For example, given the program

```c
#include <stdlib.h>

int main() {
    char* p = malloc(1024);
    *p = 0;
    free(p);
    *p = 0;
}
```

compiled with WASI-SDK via

```plain
$ /opt/wasi-sdk/bin/clang -o test.wasm test.c
```

you can observe the memory checker working like so:

```plain
$ wasmtime run -W wmemcheck ./test.wasm
Error: failed to run main module `./test.wasm`

Caused by:
    0: failed to invoke command default
    1: error while executing at wasm backtrace:
           0:  0x103 - <unknown>!__original_main
           1:   0x87 - <unknown>!_start
           2: 0x2449 - <unknown>!_start.command_export
    2: Invalid store at addr 0x10610 of size 1
```
