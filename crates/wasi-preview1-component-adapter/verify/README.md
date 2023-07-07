# verify-component-adapter

The `wasi-preview1-component-adapter` crate must compile to a wasm binary that
meets a challenging set of constraints, in order to be used as an adapter by
the `wasm-tools component new` tool.

There are a limited set of wasm sections allowed in the binary, and a limited
set of wasm modules we allow imports from.

This crate is a bin target which parses a wasm file and reports an error if it
does not fit in those constraints.
