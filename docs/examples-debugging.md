# Debugging WebAssembly

The following steps describe a common way to debug a WebAssembly module in
Wasmtime:

1. Compile your WebAssembly with debug info enabled, usually `-g`; for
   example: 

    ```sh
    clang foo.c -g -o foo.wasm
    ```

2. Run Wasmtime with the debug info enabled; this is `-g` from the CLI and
   `Config::debug_info(true)` in an embedding (e.g. see [debugging in a Rust
   embedding](./examples-rust-debugging.md))

3. Use a supported debugger:

    ```sh
    lldb -- wasmtime run -g foo.wasm
    ```
    ```sh
    gdb --args wasmtime run -g foo.wasm
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
  
