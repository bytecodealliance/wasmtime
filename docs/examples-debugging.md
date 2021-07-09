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
