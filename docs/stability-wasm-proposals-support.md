# WebAssembly Proposals Support

The following table summarizes Wasmtime's support for WebAssembly proposals as
well as the command line flag and [`wasmtime::Config`][config] method you can
use to enable or disable support for a proposal.

If a proposal is not listed, then it is not supported by Wasmtime.

Wasmtime will never enable a proposal by default unless it has reached phase 4
of [the WebAssembly standardizations process][phases] and its implementation in
Wasmtime has been [thoroughly
vetted](./contributing-implementing-wasm-proposals.html).

| WebAssembly Proposal                        | Supported in Wasmtime?           | Command Line Flag      | [`Config`][config] Method |
|---------------------------------------------|----------------------------------|------------------------|---------------------------|
| **[Import and Export Mutable Globals]**     | **Yes.**<br/>Always enabled.     | (none)                 | (none)                    |
| **[Sign-Extension Operations]**             | **Yes.**<br/>Always enabled.     | (none)                 | (none)                    |
| **[Non-Trapping Float-to-Int Conversions]** | **Yes.**<br/>Always enabled.     | (none)                 | (none)                    |
| **[Multi-Value]**                           | **Yes.**<br/>Enabled by default. | `--enable-multi-value` | [`wasm_multi_value`](https://docs.rs/wasmtime/*/wasmtime/struct.Config.html#method.wasm_multi_value) |
| **[Bulk Memory Operations]**                | **Yes.**<br/>Enabled by default. | `--enable-bulk-memory` | [`wasm_bulk_memory`](https://docs.rs/wasmtime/*/wasmtime/struct.Config.html#method.wasm_bulk_memory) |
| **[Reference Types]**                       | **Yes.**<br/>Enabled by default. | `--enable-reference-types` | [`wasm_reference_types`](https://docs.rs/wasmtime/*/wasmtime/struct.Config.html#method.wasm_reference_types) |
| **[Fixed-Width SIMD]**                      | **In progress.**                 | `--enable-simd`        | [`wasm_simd`](https://docs.rs/wasmtime/*/wasmtime/struct.Config.html#method.wasm_simd) |
| **[Threads and Atomics]**                   | **In progress.**                 | `--enable-threads`     | [`wasm_threads`](https://docs.rs/wasmtime/*/wasmtime/struct.Config.html#method.wasm_threads) |
| **[Multi-Memory]**                          | **Yes.**                         | `--enable-multi-memory`| [`wasm_multi_memory`](https://docs.rs/wasmtime/*/wasmtime/struct.Config.html#method.wasm_multi_memory) |
| **[Module Linking]**                        | **Yes.**                         | `--enable-module-linking` | [`wasm_module_linking`](https://docs.rs/wasmtime/*/wasmtime/struct.Config.html#method.wasm_module_linking) |

[config]: https://docs.rs/wasmtime/*/wasmtime/struct.Config.html
[Multi-Value]: https://github.com/WebAssembly/spec/blob/master/proposals/multi-value/Overview.md
[Bulk Memory Operations]: https://github.com/WebAssembly/bulk-memory-operations/blob/master/proposals/bulk-memory-operations/Overview.md
[Import and Export Mutable Globals]: https://github.com/WebAssembly/mutable-global/blob/master/proposals/mutable-global/Overview.md
[Reference Types]: https://github.com/WebAssembly/reference-types/blob/master/proposals/reference-types/Overview.md
[Non-Trapping Float-to-Int Conversions]: https://github.com/WebAssembly/spec/blob/master/proposals/nontrapping-float-to-int-conversion/Overview.md
[Sign-Extension Operations]: https://github.com/WebAssembly/spec/blob/master/proposals/sign-extension-ops/Overview.md
[Fixed-Width SIMD]: https://github.com/WebAssembly/simd/blob/master/proposals/simd/SIMD.md
[phases]: https://github.com/WebAssembly/meetings/blob/master/process/phases.md
[Threads and Atomics]: https://github.com/WebAssembly/threads/blob/master/proposals/threads/Overview.md
[Multi-Memory]: https://github.com/WebAssembly/multi-memory/blob/master/proposals/multi-memory/Overview.md
[Module Linking]: https://github.com/WebAssembly/module-linking/blob/master/proposals/module-linking/Explainer.md
