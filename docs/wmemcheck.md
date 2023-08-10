
Wmemcheck provides debug output for invalid mallocs, reads, and writes.

How to use:
1. When building Wasmtime, add the CLI flag "--features wmemcheck" to compile with wmemcheck configured.
    > cargo build --features wmemcheck
2. When running your wasm module, add the CLI flag "--wmemcheck".
    > wasmtime run --wmemcheck test.wasm
