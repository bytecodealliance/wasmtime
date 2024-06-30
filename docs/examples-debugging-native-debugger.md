# Debugging with `gdb` and `lldb`

The following steps describe how to use `gdb` or `lldb` to debug both the Wasm
guest and the host (i.e. the Wasmtime CLI or your Wasmtime-embedding program) at
the same time:

1. Compile your WebAssembly with debug info enabled, usually `-g`; for
   example:

    ```sh
    clang foo.c -g -o foo.wasm
    ```

2. Run Wasmtime with the debug info enabled; this is `-D debug-info` from the
   CLI and `Config::debug_info(true)` in an embedding (e.g. see [debugging in a
   Rust embedding](./examples-rust-debugging.md)). It's also recommended to use
   `-O opt-level=0` for better inspection of local variables if desired.

3. Use a supported debugger:

    ```sh
    lldb -- wasmtime run -D debug-info foo.wasm
    ```
    ```sh
    gdb --args wasmtime run -D debug-info -O opt-level=0 foo.wasm
    ```

If you run into trouble, the following discussions might help:

- On MacOS with LLDB you may need to run: `settings set
  plugin.jit-loader.gdb.enable on`
  ([#1953](https://github.com/bytecodealliance/wasmtime/issues/1953))

- With LLDB, call `__vmctx.set()` to set the current context before calling any
  dereference operators
  ([#1482](https://github.com/bytecodealliance/wasmtime/issues/1482)):
  ```sh
  (lldb) p __vmctx->set()
  (lldb) p *foo
  ```

- The address of the start of instance memory can be found in `__vmctx->memory`

- On Windows you may experience degraded WASM compilation throughput due to the
  enablement of additional native heap checks when under the debugger by default.
  You can set the environment variable `_NO_DEBUG_HEAP` to `1` to disable them.
