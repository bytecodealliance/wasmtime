# Core Dumps

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/fib-debug/main.rs

This examples shows how to configure capturing [core dumps] when a Wasm guest
traps that can then be passed to external tools (like [`wasmgdb`]) for
post-mortem analysis.

[core dumps]: https://github.com/WebAssembly/tool-conventions/blob/main/Coredump.md
[`wasmgdb`]: https://github.com/xtuc/wasm-coredump/blob/main/bin/wasmgdb/README.md

## `main.rs`

```rust,ignore
{{#include ../examples/coredump.rs}}
```
