# wasmtime-wasi-threads

Implement the `wasi-threads` [specification] in Wasmtime.

[specification]: https://github.com/WebAssembly/wasi-threads

> Note: this crate is experimental and not yet suitable for use in multi-tenant
> embeddings. As specified, a trap or WASI exit in one thread must end execution
> for all threads. Due to the complexity of stopping threads, however, this
> implementation currently exits the process entirely. This will work for some
> use cases (e.g., CLI usage) but not for embedders. This warning can be removed
> once a suitable mechanism is implemented that avoids exiting the process.
